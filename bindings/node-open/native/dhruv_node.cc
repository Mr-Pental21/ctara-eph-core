#include <node_api.h>

#include <algorithm>
#include <cstring>
#include <string>
#include <vector>

#include "dhruv.h"

namespace {

constexpr int32_t STATUS_OK = 0;
constexpr int32_t STATUS_INVALID_INPUT = 13;

#define NAPI_RETURN_IF_FAILED(env, expr) \
    do {                                  \
        napi_status _s = (expr);          \
        if (_s != napi_ok) {              \
            return nullptr;                \
        }                                 \
    } while (0)

bool GetString(napi_env env, napi_value value, std::string* out) {
    size_t len = 0;
    if (napi_get_value_string_utf8(env, value, nullptr, 0, &len) != napi_ok) {
        return false;
    }
    std::string s(len, '\0');
    if (napi_get_value_string_utf8(env, value, s.data(), len + 1, &len) != napi_ok) {
        return false;
    }
    out->assign(s.data(), len);
    return true;
}

bool GetInt32(napi_env env, napi_value value, int32_t* out) {
    return napi_get_value_int32(env, value, out) == napi_ok;
}

bool GetUint32(napi_env env, napi_value value, uint32_t* out) {
    return napi_get_value_uint32(env, value, out) == napi_ok;
}

bool GetUint64(napi_env env, napi_value value, uint64_t* out) {
    bool lossless = false;
    return napi_get_value_bigint_uint64(env, value, out, &lossless) == napi_ok;
}

bool GetDouble(napi_env env, napi_value value, double* out) {
    return napi_get_value_double(env, value, out) == napi_ok;
}

bool GetBool(napi_env env, napi_value value, bool* out) {
    return napi_get_value_bool(env, value, out) == napi_ok;
}

napi_value MakeInt32(napi_env env, int32_t value) {
    napi_value out;
    napi_create_int32(env, value, &out);
    return out;
}

napi_value MakeUint32(napi_env env, uint32_t value) {
    napi_value out;
    napi_create_uint32(env, value, &out);
    return out;
}

napi_value MakeBool(napi_env env, bool value) {
    napi_value out;
    napi_get_boolean(env, value, &out);
    return out;
}

napi_value MakeDouble(napi_env env, double value) {
    napi_value out;
    napi_create_double(env, value, &out);
    return out;
}

napi_value MakeString(napi_env env, const char* value) {
    napi_value out;
    if (value == nullptr) {
        napi_get_null(env, &out);
        return out;
    }
    napi_create_string_utf8(env, value, NAPI_AUTO_LENGTH, &out);
    return out;
}

napi_value MakeStatusResult(napi_env env, int32_t status) {
    napi_value obj;
    napi_create_object(env, &obj);
    napi_set_named_property(env, obj, "status", MakeInt32(env, status));
    return obj;
}

void SetNamed(napi_env env, napi_value obj, const char* name, napi_value value) {
    napi_set_named_property(env, obj, name, value);
}

bool GetNamedProperty(napi_env env, napi_value obj, const char* name, napi_value* out) {
    return napi_get_named_property(env, obj, name, out) == napi_ok;
}

bool GetOptionalNamedProperty(napi_env env, napi_value obj, const char* name, napi_value* out, bool* has) {
    bool present = false;
    if (napi_has_named_property(env, obj, name, &present) != napi_ok) {
        return false;
    }
    *has = present;
    if (!present) {
        return true;
    }
    return napi_get_named_property(env, obj, name, out) == napi_ok;
}

bool ReadUtcTime(napi_env env, napi_value obj, DhruvUtcTime* out) {
    napi_value v;
    if (!GetNamedProperty(env, obj, "year", &v) || !GetInt32(env, v, &out->year)) {
        return false;
    }
    if (!GetNamedProperty(env, obj, "month", &v) || !GetUint32(env, v, &out->month)) {
        return false;
    }
    if (!GetNamedProperty(env, obj, "day", &v) || !GetUint32(env, v, &out->day)) {
        return false;
    }
    if (!GetNamedProperty(env, obj, "hour", &v) || !GetUint32(env, v, &out->hour)) {
        return false;
    }
    if (!GetNamedProperty(env, obj, "minute", &v) || !GetUint32(env, v, &out->minute)) {
        return false;
    }
    if (!GetNamedProperty(env, obj, "second", &v) || !GetDouble(env, v, &out->second)) {
        return false;
    }
    return true;
}

napi_value WriteUtcTime(napi_env env, const DhruvUtcTime& utc) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "year", MakeInt32(env, utc.year));
    SetNamed(env, obj, "month", MakeUint32(env, utc.month));
    SetNamed(env, obj, "day", MakeUint32(env, utc.day));
    SetNamed(env, obj, "hour", MakeUint32(env, utc.hour));
    SetNamed(env, obj, "minute", MakeUint32(env, utc.minute));
    SetNamed(env, obj, "second", MakeDouble(env, utc.second));
    return obj;
}

bool ReadGeoLocation(napi_env env, napi_value obj, DhruvGeoLocation* out) {
    napi_value v;
    if (!GetNamedProperty(env, obj, "latitudeDeg", &v) || !GetDouble(env, v, &out->latitude_deg)) {
        return false;
    }
    if (!GetNamedProperty(env, obj, "longitudeDeg", &v) || !GetDouble(env, v, &out->longitude_deg)) {
        return false;
    }
    if (!GetNamedProperty(env, obj, "altitudeM", &v) || !GetDouble(env, v, &out->altitude_m)) {
        return false;
    }
    return true;
}

bool ReadUint8ArrayFixed(napi_env env, napi_value arr, uint8_t* out, uint32_t count) {
    bool is_array = false;
    if (napi_is_array(env, arr, &is_array) != napi_ok || !is_array) return false;
    uint32_t len = 0;
    if (napi_get_array_length(env, arr, &len) != napi_ok || len < count) return false;
    for (uint32_t i = 0; i < count; ++i) {
        napi_value v;
        uint32_t x = 0;
        if (napi_get_element(env, arr, i, &v) != napi_ok || !GetUint32(env, v, &x)) return false;
        out[i] = static_cast<uint8_t>(x);
    }
    return true;
}

bool ReadUint16ArrayFixed(napi_env env, napi_value arr, uint16_t* out, uint32_t count) {
    bool is_array = false;
    if (napi_is_array(env, arr, &is_array) != napi_ok || !is_array) return false;
    uint32_t len = 0;
    if (napi_get_array_length(env, arr, &len) != napi_ok || len < count) return false;
    for (uint32_t i = 0; i < count; ++i) {
        napi_value v;
        uint32_t x = 0;
        if (napi_get_element(env, arr, i, &v) != napi_ok || !GetUint32(env, v, &x)) return false;
        out[i] = static_cast<uint16_t>(x);
    }
    return true;
}

bool ReadDoubleArrayFixed(napi_env env, napi_value arr, double* out, uint32_t count) {
    bool is_array = false;
    if (napi_is_array(env, arr, &is_array) != napi_ok || !is_array) return false;
    uint32_t len = 0;
    if (napi_get_array_length(env, arr, &len) != napi_ok || len < count) return false;
    for (uint32_t i = 0; i < count; ++i) {
        napi_value v;
        if (napi_get_element(env, arr, i, &v) != napi_ok || !GetDouble(env, v, &out[i])) return false;
    }
    return true;
}

bool ReadRiseSetConfig(napi_env env, napi_value obj, DhruvRiseSetConfig* out) {
    napi_value v;
    bool b = false;
    if (!GetNamedProperty(env, obj, "useRefraction", &v) || !GetBool(env, v, &b)) return false;
    out->use_refraction = b ? 1 : 0;
    if (!GetNamedProperty(env, obj, "sunLimb", &v) || !GetInt32(env, v, &out->sun_limb)) return false;
    if (!GetNamedProperty(env, obj, "altitudeCorrection", &v) || !GetBool(env, v, &b)) return false;
    out->altitude_correction = b ? 1 : 0;
    return true;
}

bool ReadBhavaConfig(napi_env env, napi_value obj, DhruvBhavaConfig* out) {
    *out = dhruv_bhava_config_default();
    napi_value v;
    bool has = false;
    if (napi_has_named_property(env, obj, "system", &has) != napi_ok) return false;
    if (has && (!GetNamedProperty(env, obj, "system", &v) || !GetInt32(env, v, &out->system))) return false;
    if (napi_has_named_property(env, obj, "startingPoint", &has) != napi_ok) return false;
    if (has && (!GetNamedProperty(env, obj, "startingPoint", &v) || !GetInt32(env, v, &out->starting_point))) return false;
    if (napi_has_named_property(env, obj, "customStartDeg", &has) != napi_ok) return false;
    if (has && (!GetNamedProperty(env, obj, "customStartDeg", &v) || !GetDouble(env, v, &out->custom_start_deg))) return false;
    if (napi_has_named_property(env, obj, "referenceMode", &has) != napi_ok) return false;
    if (has && (!GetNamedProperty(env, obj, "referenceMode", &v) || !GetInt32(env, v, &out->reference_mode))) return false;
    if (napi_has_named_property(env, obj, "outputMode", &has) != napi_ok) return false;
    if (has && (!GetNamedProperty(env, obj, "outputMode", &v) || !GetInt32(env, v, &out->output_mode))) return false;
    if (napi_has_named_property(env, obj, "ayanamshaSystem", &has) != napi_ok) return false;
    if (has && (!GetNamedProperty(env, obj, "ayanamshaSystem", &v) || !GetInt32(env, v, &out->ayanamsha_system))) return false;
    if (napi_has_named_property(env, obj, "useNutation", &has) != napi_ok) return false;
    if (has) {
        bool b = false;
        if (!GetNamedProperty(env, obj, "useNutation", &v) || !GetBool(env, v, &b)) return false;
        out->use_nutation = b ? 1 : 0;
    }
    if (napi_has_named_property(env, obj, "referencePlane", &has) != napi_ok) return false;
    if (has && (!GetNamedProperty(env, obj, "referencePlane", &v) || !GetInt32(env, v, &out->reference_plane))) return false;
    return true;
}

bool ReadSankrantiConfig(napi_env env, napi_value obj, DhruvSankrantiConfig* out) {
    napi_value v;
    bool b = false;
    if (!GetNamedProperty(env, obj, "ayanamshaSystem", &v) || !GetInt32(env, v, &out->ayanamsha_system)) return false;
    if (!GetNamedProperty(env, obj, "useNutation", &v) || !GetBool(env, v, &b)) return false;
    out->use_nutation = b ? 1 : 0;
    if (!GetNamedProperty(env, obj, "referencePlane", &v) || !GetInt32(env, v, &out->reference_plane)) return false;
    if (!GetNamedProperty(env, obj, "stepSizeDays", &v) || !GetDouble(env, v, &out->step_size_days)) return false;
    if (!GetNamedProperty(env, obj, "maxIterations", &v) || !GetUint32(env, v, &out->max_iterations)) return false;
    if (!GetNamedProperty(env, obj, "convergenceDays", &v) || !GetDouble(env, v, &out->convergence_days)) return false;
    return true;
}

bool ReadDrishtiConfig(napi_env env, napi_value obj, DhruvDrishtiConfig* out) {
    napi_value v;
    bool b = false;
    if (!GetNamedProperty(env, obj, "includeBhava", &v) || !GetBool(env, v, &b)) return false;
    out->include_bhava = b ? 1 : 0;
    if (!GetNamedProperty(env, obj, "includeLagna", &v) || !GetBool(env, v, &b)) return false;
    out->include_lagna = b ? 1 : 0;
    if (!GetNamedProperty(env, obj, "includeBindus", &v) || !GetBool(env, v, &b)) return false;
    out->include_bindus = b ? 1 : 0;
    return true;
}

bool ReadGrahaPositionsConfig(napi_env env, napi_value obj, DhruvGrahaPositionsConfig* out) {
    napi_value v;
    bool b = false;
    if (!GetNamedProperty(env, obj, "includeNakshatra", &v) || !GetBool(env, v, &b)) return false;
    out->include_nakshatra = b ? 1 : 0;
    if (!GetNamedProperty(env, obj, "includeLagna", &v) || !GetBool(env, v, &b)) return false;
    out->include_lagna = b ? 1 : 0;
    if (!GetNamedProperty(env, obj, "includeOuterPlanets", &v) || !GetBool(env, v, &b)) return false;
    out->include_outer_planets = b ? 1 : 0;
    if (!GetNamedProperty(env, obj, "includeBhava", &v) || !GetBool(env, v, &b)) return false;
    out->include_bhava = b ? 1 : 0;
    return true;
}

bool ReadTimeUpagrahaConfig(napi_env env, napi_value obj, DhruvTimeUpagrahaConfig* out) {
    *out = dhruv_time_upagraha_config_default();
    napi_value v;
    bool has = false;
    uint32_t u32 = 0;
    if (!GetOptionalNamedProperty(env, obj, "gulikaPoint", &v, &has)) return false;
    if (has) {
        if (!GetUint32(env, v, &u32)) return false;
        out->gulika_point = static_cast<uint8_t>(u32);
    }
    if (!GetOptionalNamedProperty(env, obj, "maandiPoint", &v, &has)) return false;
    if (has) {
        if (!GetUint32(env, v, &u32)) return false;
        out->maandi_point = static_cast<uint8_t>(u32);
    }
    if (!GetOptionalNamedProperty(env, obj, "otherPoint", &v, &has)) return false;
    if (has) {
        if (!GetUint32(env, v, &u32)) return false;
        out->other_point = static_cast<uint8_t>(u32);
    }
    if (!GetOptionalNamedProperty(env, obj, "gulikaPlanet", &v, &has)) return false;
    if (has) {
        if (!GetUint32(env, v, &u32)) return false;
        out->gulika_planet = static_cast<uint8_t>(u32);
    }
    if (!GetOptionalNamedProperty(env, obj, "maandiPlanet", &v, &has)) return false;
    if (has) {
        if (!GetUint32(env, v, &u32)) return false;
        out->maandi_planet = static_cast<uint8_t>(u32);
    }
    return true;
}

bool ReadBindusConfig(napi_env env, napi_value obj, DhruvBindusConfig* out) {
    napi_value v;
    bool b = false;
    *out = DhruvBindusConfig{};
    out->upagraha_config = dhruv_time_upagraha_config_default();
    if (!GetNamedProperty(env, obj, "includeNakshatra", &v) || !GetBool(env, v, &b)) return false;
    out->include_nakshatra = b ? 1 : 0;
    if (!GetNamedProperty(env, obj, "includeBhava", &v) || !GetBool(env, v, &b)) return false;
    out->include_bhava = b ? 1 : 0;
    bool has = false;
    if (!GetOptionalNamedProperty(env, obj, "upagrahaConfig", &v, &has)) return false;
    if (has && !ReadTimeUpagrahaConfig(env, v, &out->upagraha_config)) return false;
    return true;
}

bool ReadAmshaChartScope(napi_env env, napi_value obj, DhruvAmshaChartScope* out) {
    napi_value v;
    bool b = false;
    if (!GetNamedProperty(env, obj, "includeBhavaCusps", &v) || !GetBool(env, v, &b)) return false;
    out->include_bhava_cusps = b ? 1 : 0;
    if (!GetNamedProperty(env, obj, "includeArudhaPadas", &v) || !GetBool(env, v, &b)) return false;
    out->include_arudha_padas = b ? 1 : 0;
    if (!GetNamedProperty(env, obj, "includeUpagrahas", &v) || !GetBool(env, v, &b)) return false;
    out->include_upagrahas = b ? 1 : 0;
    if (!GetNamedProperty(env, obj, "includeSphutas", &v) || !GetBool(env, v, &b)) return false;
    out->include_sphutas = b ? 1 : 0;
    if (!GetNamedProperty(env, obj, "includeSpecialLagnas", &v) || !GetBool(env, v, &b)) return false;
    out->include_special_lagnas = b ? 1 : 0;
    return true;
}

bool ReadAmshaSelectionConfig(napi_env env, napi_value obj, DhruvAmshaSelectionConfig* out) {
    napi_value v;
    uint32_t count = 0;
    if (!GetNamedProperty(env, obj, "count", &v) || !GetUint32(env, v, &count)) return false;
    out->count = static_cast<uint8_t>(count);
    if (!GetNamedProperty(env, obj, "codes", &v) || !ReadUint16ArrayFixed(env, v, out->codes, DHRUV_MAX_AMSHA_REQUESTS)) return false;
    if (!GetNamedProperty(env, obj, "variations", &v) || !ReadUint8ArrayFixed(env, v, out->variations, DHRUV_MAX_AMSHA_REQUESTS)) return false;
    return true;
}

bool ReadDashaSelectionConfig(napi_env env, napi_value obj, DhruvDashaSelectionConfig* out) {
    napi_value v;
    uint32_t count = 0;
    bool b = false;
    if (!GetNamedProperty(env, obj, "count", &v) || !GetUint32(env, v, &count)) return false;
    out->count = static_cast<uint8_t>(count);
    if (!GetNamedProperty(env, obj, "systems", &v) || !ReadUint8ArrayFixed(env, v, out->systems, DHRUV_MAX_DASHA_SYSTEMS)) return false;
    if (!GetNamedProperty(env, obj, "maxLevels", &v) || !ReadUint8ArrayFixed(env, v, out->max_levels, DHRUV_MAX_DASHA_SYSTEMS)) return false;
    if (!GetNamedProperty(env, obj, "maxLevel", &v) || !GetUint32(env, v, &count)) return false;
    out->max_level = static_cast<uint8_t>(count);
    if (!GetNamedProperty(env, obj, "levelMethods", &v) || !ReadUint8ArrayFixed(env, v, out->level_methods, 5)) return false;
    if (!GetNamedProperty(env, obj, "yoginiScheme", &v) || !GetUint32(env, v, &count)) return false;
    out->yogini_scheme = static_cast<uint8_t>(count);
    if (!GetNamedProperty(env, obj, "useAbhijit", &v) || !GetBool(env, v, &b)) return false;
    out->use_abhijit = b ? 1 : 0;
    if (!GetNamedProperty(env, obj, "hasSnapshotJd", &v) || !GetBool(env, v, &b)) return false;
    out->has_snapshot_jd = b ? 1 : 0;
    if (!GetNamedProperty(env, obj, "snapshotJd", &v) || !GetDouble(env, v, &out->snapshot_jd)) return false;
    return true;
}

bool ReadDashaVariationConfig(napi_env env, napi_value obj, DhruvDashaVariationConfig* out) {
    *out = dhruv_dasha_variation_config_default();
    napi_value v;
    bool has = false;

    if (!GetOptionalNamedProperty(env, obj, "levelMethods", &v, &has)) return false;
    if (has) {
        if (!ReadUint8ArrayFixed(env, v, out->level_methods, 5)) return false;
    }
    if (!GetOptionalNamedProperty(env, obj, "yoginiScheme", &v, &has)) return false;
    if (has) {
        uint32_t value = 0;
        if (!GetUint32(env, v, &value)) return false;
        out->yogini_scheme = static_cast<uint8_t>(value);
    }
    if (!GetOptionalNamedProperty(env, obj, "useAbhijit", &v, &has)) return false;
    if (has) {
        bool value = false;
        if (!GetBool(env, v, &value)) return false;
        out->use_abhijit = value ? 1 : 0;
    }
    return true;
}

bool ReadDashaPeriodValue(napi_env env, napi_value obj, DhruvDashaPeriod* out) {
    napi_value v;
    uint32_t u32 = 0;
    if (!GetNamedProperty(env, obj, "entityType", &v) || !GetUint32(env, v, &u32)) return false;
    out->entity_type = static_cast<uint8_t>(u32);
    if (!GetNamedProperty(env, obj, "entityIndex", &v) || !GetUint32(env, v, &u32)) return false;
    out->entity_index = static_cast<uint8_t>(u32);
    if (!GetNamedProperty(env, obj, "startJd", &v) || !GetDouble(env, v, &out->start_jd)) return false;
    if (!GetNamedProperty(env, obj, "endJd", &v) || !GetDouble(env, v, &out->end_jd)) return false;
    if (!GetNamedProperty(env, obj, "level", &v) || !GetUint32(env, v, &u32)) return false;
    out->level = static_cast<uint8_t>(u32);
    if (!GetNamedProperty(env, obj, "order", &v) || !GetUint32(env, v, &u32)) return false;
    out->order = static_cast<uint16_t>(u32);
    if (!GetNamedProperty(env, obj, "parentIdx", &v) || !GetUint32(env, v, &u32)) return false;
    out->parent_idx = u32;
    return true;
}

bool ReadFullKundaliConfig(napi_env env, napi_value obj, DhruvFullKundaliConfig* out) {
    *out = dhruv_full_kundali_config_default();
    napi_value v;
    bool b = false;
    uint32_t u32 = 0;
    if (!GetNamedProperty(env, obj, "includeBhavaCusps", &v) || !GetBool(env, v, &b)) return false;
    out->include_bhava_cusps = b ? 1 : 0;
    if (!GetNamedProperty(env, obj, "includeGrahaPositions", &v) || !GetBool(env, v, &b)) return false;
    out->include_graha_positions = b ? 1 : 0;
    if (!GetNamedProperty(env, obj, "includeBindus", &v) || !GetBool(env, v, &b)) return false;
    out->include_bindus = b ? 1 : 0;
    if (!GetNamedProperty(env, obj, "includeDrishti", &v) || !GetBool(env, v, &b)) return false;
    out->include_drishti = b ? 1 : 0;
    if (!GetNamedProperty(env, obj, "includeAshtakavarga", &v) || !GetBool(env, v, &b)) return false;
    out->include_ashtakavarga = b ? 1 : 0;
    if (!GetNamedProperty(env, obj, "includeUpagrahas", &v) || !GetBool(env, v, &b)) return false;
    out->include_upagrahas = b ? 1 : 0;
    if (!GetNamedProperty(env, obj, "includeSphutas", &v) || !GetBool(env, v, &b)) return false;
    out->include_sphutas = b ? 1 : 0;
    if (!GetNamedProperty(env, obj, "includeSpecialLagnas", &v) || !GetBool(env, v, &b)) return false;
    out->include_special_lagnas = b ? 1 : 0;
    if (!GetNamedProperty(env, obj, "includeAmshas", &v) || !GetBool(env, v, &b)) return false;
    out->include_amshas = b ? 1 : 0;
    if (!GetNamedProperty(env, obj, "includeShadbala", &v) || !GetBool(env, v, &b)) return false;
    out->include_shadbala = b ? 1 : 0;
    if (!GetNamedProperty(env, obj, "includeBhavaBala", &v) || !GetBool(env, v, &b)) return false;
    out->include_bhavabala = b ? 1 : 0;
    if (!GetNamedProperty(env, obj, "includeVimsopaka", &v) || !GetBool(env, v, &b)) return false;
    out->include_vimsopaka = b ? 1 : 0;
    if (!GetNamedProperty(env, obj, "includeAvastha", &v) || !GetBool(env, v, &b)) return false;
    out->include_avastha = b ? 1 : 0;
    if (!GetNamedProperty(env, obj, "includeCharakaraka", &v) || !GetBool(env, v, &b)) return false;
    out->include_charakaraka = b ? 1 : 0;
    if (!GetNamedProperty(env, obj, "charakarakaScheme", &v) || !GetUint32(env, v, &u32)) return false;
    out->charakaraka_scheme = static_cast<uint8_t>(u32);
    if (!GetNamedProperty(env, obj, "nodeDignityPolicy", &v) || !GetUint32(env, v, &out->node_dignity_policy)) return false;
    bool has = false;
    if (!GetOptionalNamedProperty(env, obj, "upagrahaConfig", &v, &has)) return false;
    if (has && !ReadTimeUpagrahaConfig(env, v, &out->upagraha_config)) return false;
    if (!GetNamedProperty(env, obj, "grahaPositionsConfig", &v) || !ReadGrahaPositionsConfig(env, v, &out->graha_positions_config)) return false;
    if (!GetNamedProperty(env, obj, "bindusConfig", &v) || !ReadBindusConfig(env, v, &out->bindus_config)) return false;
    if (!GetNamedProperty(env, obj, "drishtiConfig", &v) || !ReadDrishtiConfig(env, v, &out->drishti_config)) return false;
    if (!GetNamedProperty(env, obj, "amshaScope", &v) || !ReadAmshaChartScope(env, v, &out->amsha_scope)) return false;
    if (!GetNamedProperty(env, obj, "amshaSelection", &v) || !ReadAmshaSelectionConfig(env, v, &out->amsha_selection)) return false;
    if (!GetNamedProperty(env, obj, "includePanchang", &v) || !GetBool(env, v, &b)) return false;
    out->include_panchang = b ? 1 : 0;
    if (!GetNamedProperty(env, obj, "includeCalendar", &v) || !GetBool(env, v, &b)) return false;
    out->include_calendar = b ? 1 : 0;
    if (!GetNamedProperty(env, obj, "includeDasha", &v) || !GetBool(env, v, &b)) return false;
    out->include_dasha = b ? 1 : 0;
    if (!GetNamedProperty(env, obj, "dashaConfig", &v) || !ReadDashaSelectionConfig(env, v, &out->dasha_config)) return false;
    return true;
}

bool ReadSphutalInputs(napi_env env, napi_value obj, DhruvSphutalInputs* out) {
    napi_value v;
    if (!GetNamedProperty(env, obj, "sun", &v) || !GetDouble(env, v, &out->sun)) return false;
    if (!GetNamedProperty(env, obj, "moon", &v) || !GetDouble(env, v, &out->moon)) return false;
    if (!GetNamedProperty(env, obj, "mars", &v) || !GetDouble(env, v, &out->mars)) return false;
    if (!GetNamedProperty(env, obj, "jupiter", &v) || !GetDouble(env, v, &out->jupiter)) return false;
    if (!GetNamedProperty(env, obj, "venus", &v) || !GetDouble(env, v, &out->venus)) return false;
    if (!GetNamedProperty(env, obj, "rahu", &v) || !GetDouble(env, v, &out->rahu)) return false;
    if (!GetNamedProperty(env, obj, "lagna", &v) || !GetDouble(env, v, &out->lagna)) return false;
    if (!GetNamedProperty(env, obj, "eighthLord", &v) || !GetDouble(env, v, &out->eighth_lord)) return false;
    if (!GetNamedProperty(env, obj, "gulika", &v) || !GetDouble(env, v, &out->gulika)) return false;
    return true;
}

bool ReadBhinnaAshtakavarga(napi_env env, napi_value obj, DhruvBhinnaAshtakavarga* out) {
    napi_value v;
    uint32_t idx = 0;
    if (!GetNamedProperty(env, obj, "grahaIndex", &v) || !GetUint32(env, v, &idx)) return false;
    out->graha_index = static_cast<uint8_t>(idx);
    if (!GetNamedProperty(env, obj, "points", &v) || !ReadUint8ArrayFixed(env, v, out->points, 12)) return false;
    for (uint32_t i = 0; i < 12; ++i) {
        for (uint32_t j = 0; j < 8; ++j) {
            out->contributors[i][j] = 0;
        }
    }
    bool hasContributors = false;
    if (!GetOptionalNamedProperty(env, obj, "contributors", &v, &hasContributors)) return false;
    if (!hasContributors) return true;

    uint32_t rows = 0;
    if (napi_get_array_length(env, v, &rows) != napi_ok || rows != 12) return false;
    for (uint32_t i = 0; i < 12; ++i) {
        napi_value row;
        if (napi_get_element(env, v, i, &row) != napi_ok) return false;
        uint32_t cols = 0;
        if (napi_get_array_length(env, row, &cols) != napi_ok || cols != 8) return false;
        for (uint32_t j = 0; j < 8; ++j) {
            napi_value item;
            uint32_t val = 0;
            if (napi_get_element(env, row, j, &item) != napi_ok || !GetUint32(env, item, &val)) return false;
            out->contributors[i][j] = static_cast<uint8_t>(val);
        }
    }
    return true;
}

bool ReadBhavaBalaInputs(napi_env env, napi_value obj, DhruvBhavaBalaInputs* out) {
    napi_value v;
    uint32_t u32 = 0;
    if (!GetNamedProperty(env, obj, "cuspSiderealLons", &v) || !ReadDoubleArrayFixed(env, v, out->cusp_sidereal_lons, 12)) return false;
    if (!GetNamedProperty(env, obj, "ascendantSiderealLon", &v) || !GetDouble(env, v, &out->ascendant_sidereal_lon)) return false;
    if (!GetNamedProperty(env, obj, "meridianSiderealLon", &v) || !GetDouble(env, v, &out->meridian_sidereal_lon)) return false;
    if (!GetNamedProperty(env, obj, "grahaBhavaNumbers", &v) || !ReadUint8ArrayFixed(env, v, out->graha_bhava_numbers, DHRUV_GRAHA_COUNT)) return false;
    if (!GetNamedProperty(env, obj, "houseLordStrengths", &v) || !ReadDoubleArrayFixed(env, v, out->house_lord_strengths, 12)) return false;
    if (!GetNamedProperty(env, obj, "aspectVirupas", &v)) return false;
    for (uint32_t i = 0; i < DHRUV_GRAHA_COUNT; ++i) {
        napi_value row;
        if (napi_get_element(env, v, i, &row) != napi_ok) return false;
        if (!ReadDoubleArrayFixed(env, row, out->aspect_virupas[i], 12)) return false;
    }
    if (!GetNamedProperty(env, obj, "birthPeriod", &v) || !GetUint32(env, v, &u32)) return false;
    out->birth_period = u32;
    return true;
}

bool ReadExternalPtr(napi_env env, napi_value value, void** out) {
    return napi_get_value_external(env, value, out) == napi_ok;
}

napi_value MakeExternalPtr(napi_env env, void* ptr) {
    napi_value out;
    napi_create_external(env, ptr, nullptr, nullptr, &out);
    return out;
}

napi_value WriteStateVector(napi_env env, const DhruvStateVector& vec) {
    napi_value obj;
    napi_create_object(env, &obj);

    napi_value pos;
    napi_create_array_with_length(env, 3, &pos);
    for (uint32_t i = 0; i < 3; ++i) {
        napi_set_element(env, pos, i, MakeDouble(env, vec.position_km[i]));
    }

    napi_value vel;
    napi_create_array_with_length(env, 3, &vel);
    for (uint32_t i = 0; i < 3; ++i) {
        napi_set_element(env, vel, i, MakeDouble(env, vec.velocity_km_s[i]));
    }

    SetNamed(env, obj, "positionKm", pos);
    SetNamed(env, obj, "velocityKmS", vel);
    return obj;
}

napi_value WriteSphericalState(napi_env env, const DhruvSphericalState& st) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "lonDeg", MakeDouble(env, st.lon_deg));
    SetNamed(env, obj, "latDeg", MakeDouble(env, st.lat_deg));
    SetNamed(env, obj, "distanceKm", MakeDouble(env, st.distance_km));
    SetNamed(env, obj, "lonSpeed", MakeDouble(env, st.lon_speed));
    SetNamed(env, obj, "latSpeed", MakeDouble(env, st.lat_speed));
    SetNamed(env, obj, "distanceSpeed", MakeDouble(env, st.distance_speed));
    return obj;
}

napi_value WriteQueryResult(napi_env env, const DhruvQueryResult& result, int32_t output_mode) {
    napi_value obj;
    napi_create_object(env, &obj);

    napi_value nullv;
    napi_get_null(env, &nullv);

    if (output_mode == DHRUV_QUERY_OUTPUT_SPHERICAL) {
        SetNamed(env, obj, "state", nullv);
    } else {
        SetNamed(env, obj, "state", WriteStateVector(env, result.state_vector));
    }

    if (output_mode == DHRUV_QUERY_OUTPUT_CARTESIAN) {
        SetNamed(env, obj, "sphericalState", nullv);
    } else {
        SetNamed(env, obj, "sphericalState", WriteSphericalState(env, result.spherical_state));
    }

    SetNamed(env, obj, "outputMode", MakeInt32(env, output_mode));
    return obj;
}

bool ReadQueryRequest(napi_env env, napi_value obj, DhruvQueryRequest* out) {
    *out = DhruvQueryRequest{};

    napi_value v;
    if (!GetNamedProperty(env, obj, "target", &v) || !GetInt32(env, v, &out->target)) return false;
    if (!GetNamedProperty(env, obj, "observer", &v) || !GetInt32(env, v, &out->observer)) return false;

    bool has = false;
    if (!GetOptionalNamedProperty(env, obj, "frame", &v, &has)) return false;
    out->frame = 0;
    if (has && !GetInt32(env, v, &out->frame)) return false;

    out->output_mode = DHRUV_QUERY_OUTPUT_CARTESIAN;
    if (!GetOptionalNamedProperty(env, obj, "outputMode", &v, &has)) return false;
    if (has && !GetInt32(env, v, &out->output_mode)) return false;

    bool has_time_kind = false;
    if (!GetOptionalNamedProperty(env, obj, "timeKind", &v, &has_time_kind)) return false;
    if (has_time_kind && !GetInt32(env, v, &out->time_kind)) return false;

    bool has_epoch = false;
    if (!GetOptionalNamedProperty(env, obj, "epochTdbJd", &v, &has_epoch)) return false;
    if (has_epoch && !GetDouble(env, v, &out->epoch_tdb_jd)) return false;

    bool has_utc = false;
    if (!GetOptionalNamedProperty(env, obj, "utc", &v, &has_utc)) return false;
    if (has_utc && !ReadUtcTime(env, v, &out->utc)) return false;

    if (!has_time_kind) {
        if (has_epoch == has_utc) return false;
        out->time_kind = has_utc ? DHRUV_QUERY_TIME_UTC : DHRUV_QUERY_TIME_JD_TDB;
    }

    if (out->time_kind == DHRUV_QUERY_TIME_UTC) return has_utc;
    if (out->time_kind == DHRUV_QUERY_TIME_JD_TDB) return has_epoch;
    return true;
}

napi_value WriteLunarPhaseEvent(napi_env env, const DhruvLunarPhaseEvent& ev) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "utc", WriteUtcTime(env, ev.utc));
    SetNamed(env, obj, "phase", MakeInt32(env, ev.phase));
    SetNamed(env, obj, "moonLongitudeDeg", MakeDouble(env, ev.moon_longitude_deg));
    SetNamed(env, obj, "sunLongitudeDeg", MakeDouble(env, ev.sun_longitude_deg));
    return obj;
}

napi_value WriteTithiInfo(napi_env env, const DhruvTithiInfo& t) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "tithiIndex", MakeInt32(env, t.tithi_index));
    SetNamed(env, obj, "paksha", MakeInt32(env, t.paksha));
    SetNamed(env, obj, "tithiInPaksha", MakeInt32(env, t.tithi_in_paksha));
    SetNamed(env, obj, "start", WriteUtcTime(env, t.start));
    SetNamed(env, obj, "end", WriteUtcTime(env, t.end));
    return obj;
}

napi_value WriteVaarInfo(napi_env env, const DhruvVaarInfo& v) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "vaarIndex", MakeInt32(env, v.vaar_index));
    SetNamed(env, obj, "start", WriteUtcTime(env, v.start));
    SetNamed(env, obj, "end", WriteUtcTime(env, v.end));
    return obj;
}

napi_value WriteKaranaInfo(napi_env env, const DhruvKaranaInfo& k) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "karanaIndex", MakeInt32(env, k.karana_index));
    SetNamed(env, obj, "karanaNameIndex", MakeInt32(env, k.karana_name_index));
    SetNamed(env, obj, "start", WriteUtcTime(env, k.start));
    SetNamed(env, obj, "end", WriteUtcTime(env, k.end));
    return obj;
}

napi_value WriteYogaInfo(napi_env env, const DhruvYogaInfo& y) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "yogaIndex", MakeInt32(env, y.yoga_index));
    SetNamed(env, obj, "start", WriteUtcTime(env, y.start));
    SetNamed(env, obj, "end", WriteUtcTime(env, y.end));
    return obj;
}

napi_value WriteHoraInfo(napi_env env, const DhruvHoraInfo& h) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "horaIndex", MakeInt32(env, h.hora_index));
    SetNamed(env, obj, "horaPosition", MakeInt32(env, h.hora_position));
    SetNamed(env, obj, "start", WriteUtcTime(env, h.start));
    SetNamed(env, obj, "end", WriteUtcTime(env, h.end));
    return obj;
}

napi_value WriteGhatikaInfo(napi_env env, const DhruvGhatikaInfo& g) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "value", MakeInt32(env, g.value));
    SetNamed(env, obj, "start", WriteUtcTime(env, g.start));
    SetNamed(env, obj, "end", WriteUtcTime(env, g.end));
    return obj;
}

napi_value WritePanchangNakshatraInfo(napi_env env, const DhruvPanchangNakshatraInfo& n) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "nakshatraIndex", MakeInt32(env, n.nakshatra_index));
    SetNamed(env, obj, "pada", MakeInt32(env, n.pada));
    SetNamed(env, obj, "start", WriteUtcTime(env, n.start));
    SetNamed(env, obj, "end", WriteUtcTime(env, n.end));
    return obj;
}

napi_value WriteMasaInfo(napi_env env, const DhruvMasaInfo& m) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "masaIndex", MakeInt32(env, m.masa_index));
    SetNamed(env, obj, "adhika", MakeBool(env, m.adhika != 0));
    SetNamed(env, obj, "start", WriteUtcTime(env, m.start));
    SetNamed(env, obj, "end", WriteUtcTime(env, m.end));
    return obj;
}

napi_value WriteAyanaInfo(napi_env env, const DhruvAyanaInfo& a) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "ayana", MakeInt32(env, a.ayana));
    SetNamed(env, obj, "start", WriteUtcTime(env, a.start));
    SetNamed(env, obj, "end", WriteUtcTime(env, a.end));
    return obj;
}

napi_value WriteVarshaInfo(napi_env env, const DhruvVarshaInfo& v) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "samvatsaraIndex", MakeInt32(env, v.samvatsara_index));
    SetNamed(env, obj, "order", MakeInt32(env, v.order));
    SetNamed(env, obj, "start", WriteUtcTime(env, v.start));
    SetNamed(env, obj, "end", WriteUtcTime(env, v.end));
    return obj;
}

bool ReadPanchangComputeRequest(napi_env env, napi_value obj, DhruvPanchangComputeRequest* out) {
    napi_value v;
    if (!GetNamedProperty(env, obj, "timeKind", &v) || !GetInt32(env, v, &out->time_kind)) return false;
    if (!GetNamedProperty(env, obj, "jdTdb", &v) || !GetDouble(env, v, &out->jd_tdb)) return false;
    if (!GetNamedProperty(env, obj, "utc", &v) || !ReadUtcTime(env, v, &out->utc)) return false;
    if (!GetNamedProperty(env, obj, "includeMask", &v) || !GetUint32(env, v, &out->include_mask)) return false;
    if (!GetNamedProperty(env, obj, "location", &v) || !ReadGeoLocation(env, v, &out->location)) return false;
    if (!GetNamedProperty(env, obj, "riseSetConfig", &v) || !ReadRiseSetConfig(env, v, &out->riseset_config)) return false;
    if (!GetNamedProperty(env, obj, "sankrantiConfig", &v) || !ReadSankrantiConfig(env, v, &out->sankranti_config)) return false;
    return true;
}

napi_value WritePanchangOperationResult(napi_env env, const DhruvPanchangOperationResult& p) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "tithiValid", MakeBool(env, p.tithi_valid != 0));
    SetNamed(env, obj, "tithi", WriteTithiInfo(env, p.tithi));
    SetNamed(env, obj, "karanaValid", MakeBool(env, p.karana_valid != 0));
    SetNamed(env, obj, "karana", WriteKaranaInfo(env, p.karana));
    SetNamed(env, obj, "yogaValid", MakeBool(env, p.yoga_valid != 0));
    SetNamed(env, obj, "yoga", WriteYogaInfo(env, p.yoga));
    SetNamed(env, obj, "vaarValid", MakeBool(env, p.vaar_valid != 0));
    SetNamed(env, obj, "vaar", WriteVaarInfo(env, p.vaar));
    SetNamed(env, obj, "horaValid", MakeBool(env, p.hora_valid != 0));
    SetNamed(env, obj, "hora", WriteHoraInfo(env, p.hora));
    SetNamed(env, obj, "ghatikaValid", MakeBool(env, p.ghatika_valid != 0));
    SetNamed(env, obj, "ghatika", WriteGhatikaInfo(env, p.ghatika));
    SetNamed(env, obj, "nakshatraValid", MakeBool(env, p.nakshatra_valid != 0));
    SetNamed(env, obj, "nakshatra", WritePanchangNakshatraInfo(env, p.nakshatra));
    SetNamed(env, obj, "masaValid", MakeBool(env, p.masa_valid != 0));
    SetNamed(env, obj, "masa", WriteMasaInfo(env, p.masa));
    SetNamed(env, obj, "ayanaValid", MakeBool(env, p.ayana_valid != 0));
    SetNamed(env, obj, "ayana", WriteAyanaInfo(env, p.ayana));
    SetNamed(env, obj, "varshaValid", MakeBool(env, p.varsha_valid != 0));
    SetNamed(env, obj, "varsha", WriteVarshaInfo(env, p.varsha));
    return obj;
}

napi_value WriteDashaPeriod(napi_env env, const DhruvDashaPeriod& p) {
    napi_value po;
    napi_create_object(env, &po);
    SetNamed(env, po, "entityType", MakeUint32(env, p.entity_type));
    SetNamed(env, po, "entityIndex", MakeUint32(env, p.entity_index));
    SetNamed(env, po, "entityName", MakeString(env, p.entity_name ? p.entity_name : ""));
    SetNamed(env, po, "startJd", MakeDouble(env, p.start_jd));
    SetNamed(env, po, "endJd", MakeDouble(env, p.end_jd));
    SetNamed(env, po, "level", MakeUint32(env, p.level));
    SetNamed(env, po, "order", MakeUint32(env, p.order));
    SetNamed(env, po, "parentIdx", MakeUint32(env, p.parent_idx));
    return po;
}

napi_value WriteDashaVariationConfig(napi_env env, const DhruvDashaVariationConfig& cfg) {
    napi_value obj;
    napi_create_object(env, &obj);
    napi_value methods;
    napi_create_array_with_length(env, 5, &methods);
    for (uint32_t i = 0; i < 5; ++i) {
        napi_set_element(env, methods, i, MakeUint32(env, cfg.level_methods[i]));
    }
    SetNamed(env, obj, "levelMethods", methods);
    SetNamed(env, obj, "yoginiScheme", MakeUint32(env, cfg.yogini_scheme));
    SetNamed(env, obj, "useAbhijit", MakeBool(env, cfg.use_abhijit != 0));
    return obj;
}

int32_t WriteDashaPeriodList(napi_env env, DhruvDashaPeriodListHandle handle, napi_value* out) {
    uint32_t count = 0;
    int32_t status = dhruv_dasha_period_list_count(handle, &count);
    if (status != STATUS_OK) {
        dhruv_dasha_period_list_free(handle);
        return status;
    }

    napi_value periods;
    napi_create_array_with_length(env, count, &periods);
    for (uint32_t idx = 0; idx < count; ++idx) {
        DhruvDashaPeriod period{};
        status = dhruv_dasha_period_list_at(handle, idx, &period);
        if (status != STATUS_OK) {
            dhruv_dasha_period_list_free(handle);
            return status;
        }
        napi_set_element(env, periods, idx, WriteDashaPeriod(env, period));
    }

    dhruv_dasha_period_list_free(handle);
    *out = periods;
    return STATUS_OK;
}

napi_value WriteFullPanchangInfo(napi_env env, const DhruvPanchangInfo& p) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "tithi", WriteTithiInfo(env, p.tithi));
    SetNamed(env, obj, "karana", WriteKaranaInfo(env, p.karana));
    SetNamed(env, obj, "yoga", WriteYogaInfo(env, p.yoga));
    SetNamed(env, obj, "vaar", WriteVaarInfo(env, p.vaar));
    SetNamed(env, obj, "hora", WriteHoraInfo(env, p.hora));
    SetNamed(env, obj, "ghatika", WriteGhatikaInfo(env, p.ghatika));
    SetNamed(env, obj, "nakshatra", WritePanchangNakshatraInfo(env, p.nakshatra));
    SetNamed(env, obj, "calendarValid", MakeBool(env, p.calendar_valid != 0));
    SetNamed(env, obj, "masa", WriteMasaInfo(env, p.masa));
    SetNamed(env, obj, "ayana", WriteAyanaInfo(env, p.ayana));
    SetNamed(env, obj, "varsha", WriteVarshaInfo(env, p.varsha));
    return obj;
}

int32_t WriteDashaHierarchyFromHandle(
    napi_env env,
    DhruvDashaHierarchyHandle handle,
    uint8_t system,
    napi_value* out) {
    uint8_t level_count = 0;
    int32_t status = dhruv_dasha_hierarchy_level_count(handle, &level_count);
    if (status != STATUS_OK) {
        return status;
    }

    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "system", MakeUint32(env, system));

    napi_value levels;
    napi_create_array_with_length(env, level_count, &levels);
    for (uint32_t lvl = 0; lvl < level_count; ++lvl) {
        uint32_t period_count = 0;
        status = dhruv_dasha_hierarchy_period_count(handle, static_cast<uint8_t>(lvl), &period_count);
        if (status != STATUS_OK) {
            return status;
        }

        napi_value level_obj;
        napi_create_object(env, &level_obj);
        SetNamed(env, level_obj, "level", MakeUint32(env, lvl));

        napi_value periods;
        napi_create_array_with_length(env, period_count, &periods);
        for (uint32_t idx = 0; idx < period_count; ++idx) {
            DhruvDashaPeriod period{};
            status = dhruv_dasha_hierarchy_period_at(
                handle,
                static_cast<uint8_t>(lvl),
                idx,
                &period);
            if (status != STATUS_OK) {
                return status;
            }
            napi_set_element(env, periods, idx, WriteDashaPeriod(env, period));
        }

        SetNamed(env, level_obj, "periods", periods);
        napi_set_element(env, levels, lvl, level_obj);
    }

    SetNamed(env, obj, "levels", levels);
    *out = obj;
    return STATUS_OK;
}

napi_value WriteSpecialLagnas(napi_env env, const DhruvSpecialLagnas& s) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "bhavaLagna", MakeDouble(env, s.bhava_lagna));
    SetNamed(env, obj, "horaLagna", MakeDouble(env, s.hora_lagna));
    SetNamed(env, obj, "ghatiLagna", MakeDouble(env, s.ghati_lagna));
    SetNamed(env, obj, "vighatiLagna", MakeDouble(env, s.vighati_lagna));
    SetNamed(env, obj, "varnadaLagna", MakeDouble(env, s.varnada_lagna));
    SetNamed(env, obj, "sreeLagna", MakeDouble(env, s.sree_lagna));
    SetNamed(env, obj, "pranapadaLagna", MakeDouble(env, s.pranapada_lagna));
    SetNamed(env, obj, "induLagna", MakeDouble(env, s.indu_lagna));
    return obj;
}

napi_value WriteArudhaResult(napi_env env, const DhruvArudhaResult& a) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "bhavaNumber", MakeUint32(env, a.bhava_number));
    SetNamed(env, obj, "longitudeDeg", MakeDouble(env, a.longitude_deg));
    SetNamed(env, obj, "rashiIndex", MakeUint32(env, a.rashi_index));
    return obj;
}

napi_value WriteAllUpagrahas(napi_env env, const DhruvAllUpagrahas& u) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "gulika", MakeDouble(env, u.gulika));
    SetNamed(env, obj, "maandi", MakeDouble(env, u.maandi));
    SetNamed(env, obj, "kaala", MakeDouble(env, u.kaala));
    SetNamed(env, obj, "mrityu", MakeDouble(env, u.mrityu));
    SetNamed(env, obj, "arthaPrahara", MakeDouble(env, u.artha_prahara));
    SetNamed(env, obj, "yamaGhantaka", MakeDouble(env, u.yama_ghantaka));
    SetNamed(env, obj, "dhooma", MakeDouble(env, u.dhooma));
    SetNamed(env, obj, "vyatipata", MakeDouble(env, u.vyatipata));
    SetNamed(env, obj, "parivesha", MakeDouble(env, u.parivesha));
    SetNamed(env, obj, "indraChapa", MakeDouble(env, u.indra_chapa));
    SetNamed(env, obj, "upaketu", MakeDouble(env, u.upaketu));
    return obj;
}

napi_value WriteTimeUpagrahaConfig(napi_env env, const DhruvTimeUpagrahaConfig& cfg) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "gulikaPoint", MakeUint32(env, cfg.gulika_point));
    SetNamed(env, obj, "maandiPoint", MakeUint32(env, cfg.maandi_point));
    SetNamed(env, obj, "otherPoint", MakeUint32(env, cfg.other_point));
    SetNamed(env, obj, "gulikaPlanet", MakeUint32(env, cfg.gulika_planet));
    SetNamed(env, obj, "maandiPlanet", MakeUint32(env, cfg.maandi_planet));
    return obj;
}

napi_value WriteShadbalaResult(napi_env env, const DhruvShadbalaResult& s) {
    napi_value obj;
    napi_create_object(env, &obj);
    napi_value entries;
    napi_create_array_with_length(env, 7, &entries);
    napi_value totals;
    napi_create_array_with_length(env, 7, &totals);
    for (uint32_t i = 0; i < 7; ++i) {
        const DhruvShadbalaEntry& e = s.entries[i];
        napi_value eo;
        napi_create_object(env, &eo);
        SetNamed(env, eo, "grahaIndex", MakeUint32(env, e.graha_index));

        napi_value sthana;
        napi_create_object(env, &sthana);
        SetNamed(env, sthana, "uchcha", MakeDouble(env, e.sthana.uchcha));
        SetNamed(env, sthana, "saptavargaja", MakeDouble(env, e.sthana.saptavargaja));
        SetNamed(env, sthana, "ojhayugma", MakeDouble(env, e.sthana.ojhayugma));
        SetNamed(env, sthana, "kendradi", MakeDouble(env, e.sthana.kendradi));
        SetNamed(env, sthana, "drekkana", MakeDouble(env, e.sthana.drekkana));
        SetNamed(env, sthana, "total", MakeDouble(env, e.sthana.total));
        SetNamed(env, eo, "sthana", sthana);

        SetNamed(env, eo, "dig", MakeDouble(env, e.dig));

        napi_value kala;
        napi_create_object(env, &kala);
        SetNamed(env, kala, "nathonnatha", MakeDouble(env, e.kala.nathonnatha));
        SetNamed(env, kala, "paksha", MakeDouble(env, e.kala.paksha));
        SetNamed(env, kala, "tribhaga", MakeDouble(env, e.kala.tribhaga));
        SetNamed(env, kala, "abda", MakeDouble(env, e.kala.abda));
        SetNamed(env, kala, "masa", MakeDouble(env, e.kala.masa));
        SetNamed(env, kala, "vara", MakeDouble(env, e.kala.vara));
        SetNamed(env, kala, "hora", MakeDouble(env, e.kala.hora));
        SetNamed(env, kala, "ayana", MakeDouble(env, e.kala.ayana));
        SetNamed(env, kala, "yuddha", MakeDouble(env, e.kala.yuddha));
        SetNamed(env, kala, "total", MakeDouble(env, e.kala.total));
        SetNamed(env, eo, "kala", kala);

        SetNamed(env, eo, "cheshta", MakeDouble(env, e.cheshta));
        SetNamed(env, eo, "naisargika", MakeDouble(env, e.naisargika));
        SetNamed(env, eo, "drik", MakeDouble(env, e.drik));
        SetNamed(env, eo, "totalShashtiamsas", MakeDouble(env, e.total_shashtiamsas));
        SetNamed(env, eo, "totalRupas", MakeDouble(env, e.total_rupas));
        SetNamed(env, eo, "requiredStrength", MakeDouble(env, e.required_strength));
        SetNamed(env, eo, "isStrong", MakeBool(env, e.is_strong != 0));

        napi_set_element(env, entries, i, eo);
        napi_set_element(env, totals, i, MakeDouble(env, e.total_rupas));
    }
    SetNamed(env, obj, "entries", entries);
    SetNamed(env, obj, "totalRupas", totals);
    return obj;
}

napi_value WriteBhavaBalaResult(napi_env env, const DhruvBhavaBalaResult& b) {
    napi_value obj;
    napi_create_object(env, &obj);
    napi_value entries;
    napi_create_array_with_length(env, 12, &entries);
    for (uint32_t i = 0; i < 12; ++i) {
        const DhruvBhavaBalaEntry& e = b.entries[i];
        napi_value eo;
        napi_create_object(env, &eo);
        SetNamed(env, eo, "bhavaNumber", MakeUint32(env, e.bhava_number));
        SetNamed(env, eo, "cuspSiderealLon", MakeDouble(env, e.cusp_sidereal_lon));
        SetNamed(env, eo, "rashiIndex", MakeUint32(env, e.rashi_index));
        SetNamed(env, eo, "lordGrahaIndex", MakeUint32(env, e.lord_graha_index));
        SetNamed(env, eo, "bhavadhipati", MakeDouble(env, e.bhavadhipati));
        SetNamed(env, eo, "dig", MakeDouble(env, e.dig));
        SetNamed(env, eo, "drishti", MakeDouble(env, e.drishti));
        SetNamed(env, eo, "occupationBonus", MakeDouble(env, e.occupation_bonus));
        SetNamed(env, eo, "risingBonus", MakeDouble(env, e.rising_bonus));
        SetNamed(env, eo, "totalVirupas", MakeDouble(env, e.total_virupas));
        SetNamed(env, eo, "totalRupas", MakeDouble(env, e.total_rupas));
        napi_set_element(env, entries, i, eo);
    }
    SetNamed(env, obj, "entries", entries);
    return obj;
}

napi_value WriteVimsopakaResult(napi_env env, const DhruvVimsopakaResult& v) {
    napi_value obj;
    napi_create_object(env, &obj);
    napi_value entries;
    napi_create_array_with_length(env, DHRUV_GRAHA_COUNT, &entries);
    for (uint32_t i = 0; i < DHRUV_GRAHA_COUNT; ++i) {
        const DhruvVimsopakaEntry& e = v.entries[i];
        napi_value eo;
        napi_create_object(env, &eo);
        SetNamed(env, eo, "grahaIndex", MakeUint32(env, e.graha_index));
        SetNamed(env, eo, "shadvarga", MakeDouble(env, e.shadvarga));
        SetNamed(env, eo, "saptavarga", MakeDouble(env, e.saptavarga));
        SetNamed(env, eo, "dashavarga", MakeDouble(env, e.dashavarga));
        SetNamed(env, eo, "shodasavarga", MakeDouble(env, e.shodasavarga));
        napi_set_element(env, entries, i, eo);
    }
    SetNamed(env, obj, "entries", entries);
    return obj;
}

napi_value WriteAllGrahaAvasthas(napi_env env, const DhruvAllGrahaAvasthas& a) {
    napi_value obj;
    napi_create_object(env, &obj);
    napi_value entries;
    napi_create_array_with_length(env, DHRUV_GRAHA_COUNT, &entries);
    for (uint32_t i = 0; i < DHRUV_GRAHA_COUNT; ++i) {
        const DhruvGrahaAvasthas& e = a.entries[i];
        napi_value eo;
        napi_create_object(env, &eo);
        SetNamed(env, eo, "baladi", MakeUint32(env, e.baladi));
        SetNamed(env, eo, "jagradadi", MakeUint32(env, e.jagradadi));
        SetNamed(env, eo, "deeptadi", MakeUint32(env, e.deeptadi));
        SetNamed(env, eo, "lajjitadi", MakeUint32(env, e.lajjitadi));

        napi_value sayanadi;
        napi_create_object(env, &sayanadi);
        SetNamed(env, sayanadi, "avastha", MakeUint32(env, e.sayanadi.avastha));
        napi_value sub;
        napi_create_array_with_length(env, 5, &sub);
        for (uint32_t j = 0; j < 5; ++j) {
            napi_set_element(env, sub, j, MakeUint32(env, e.sayanadi.sub_states[j]));
        }
        SetNamed(env, sayanadi, "subStates", sub);
        SetNamed(env, eo, "sayanadi", sayanadi);
        napi_set_element(env, entries, i, eo);
    }
    SetNamed(env, obj, "entries", entries);
    return obj;
}

napi_value WriteCharakarakaResult(napi_env env, const DhruvCharakarakaResult& c) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "scheme", MakeUint32(env, c.scheme));
    SetNamed(env, obj, "usedEightKarakas", MakeBool(env, c.used_eight_karakas != 0));
    SetNamed(env, obj, "count", MakeUint32(env, c.count));

    napi_value entries;
    napi_create_array_with_length(env, c.count, &entries);
    for (uint32_t i = 0; i < c.count && i < DHRUV_MAX_CHARAKARAKA_ENTRIES; ++i) {
        const DhruvCharakarakaEntry& e = c.entries[i];
        napi_value eo;
        napi_create_object(env, &eo);
        SetNamed(env, eo, "roleCode", MakeUint32(env, e.role_code));
        SetNamed(env, eo, "grahaIndex", MakeUint32(env, e.graha_index));
        SetNamed(env, eo, "rank", MakeUint32(env, e.rank));
        SetNamed(env, eo, "longitudeDeg", MakeDouble(env, e.longitude_deg));
        SetNamed(env, eo, "degreesInRashi", MakeDouble(env, e.degrees_in_rashi));
        SetNamed(env, eo, "effectiveDegreesInRashi", MakeDouble(env, e.effective_degrees_in_rashi));
        napi_set_element(env, entries, i, eo);
    }
    SetNamed(env, obj, "entries", entries);
    return obj;
}

napi_value WriteSphutalResult(napi_env env, const DhruvSphutalResult& s) {
    napi_value obj;
    napi_create_object(env, &obj);
    napi_value arr;
    napi_create_array_with_length(env, DHRUV_SPHUTA_COUNT, &arr);
    for (uint32_t i = 0; i < DHRUV_SPHUTA_COUNT; ++i) {
        napi_set_element(env, arr, i, MakeDouble(env, s.longitudes[i]));
    }
    SetNamed(env, obj, "longitudes", arr);
    return obj;
}

napi_value WriteDrishtiEntry(napi_env env, const DhruvDrishtiEntry& d) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "angularDistance", MakeDouble(env, d.angular_distance));
    SetNamed(env, obj, "baseVirupa", MakeDouble(env, d.base_virupa));
    SetNamed(env, obj, "specialVirupa", MakeDouble(env, d.special_virupa));
    SetNamed(env, obj, "totalVirupa", MakeDouble(env, d.total_virupa));
    return obj;
}

napi_value WriteGrahaEntry(napi_env env, const DhruvGrahaEntry& g) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "siderealLongitude", MakeDouble(env, g.sidereal_longitude));
    SetNamed(env, obj, "rashiIndex", MakeUint32(env, g.rashi_index));
    SetNamed(env, obj, "nakshatraIndex", MakeUint32(env, g.nakshatra_index));
    SetNamed(env, obj, "pada", MakeUint32(env, g.pada));
    SetNamed(env, obj, "bhavaNumber", MakeUint32(env, g.bhava_number));
    return obj;
}

napi_value WriteBhinnaAshtakavarga(napi_env env, const DhruvBhinnaAshtakavarga& b) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "grahaIndex", MakeUint32(env, b.graha_index));
    napi_value points;
    napi_create_array_with_length(env, 12, &points);
    for (uint32_t i = 0; i < 12; ++i) {
        napi_set_element(env, points, i, MakeUint32(env, b.points[i]));
    }
    SetNamed(env, obj, "points", points);
    napi_value contributors;
    napi_create_array_with_length(env, 12, &contributors);
    for (uint32_t i = 0; i < 12; ++i) {
        napi_value row;
        napi_create_array_with_length(env, 8, &row);
        for (uint32_t j = 0; j < 8; ++j) {
            napi_set_element(env, row, j, MakeUint32(env, b.contributors[i][j]));
        }
        napi_set_element(env, contributors, i, row);
    }
    SetNamed(env, obj, "contributors", contributors);
    return obj;
}

napi_value WriteSarvaAshtakavarga(napi_env env, const DhruvSarvaAshtakavarga& s) {
    napi_value obj;
    napi_create_object(env, &obj);
    napi_value total;
    napi_create_array_with_length(env, 12, &total);
    napi_value trikona;
    napi_create_array_with_length(env, 12, &trikona);
    napi_value ekadhipatya;
    napi_create_array_with_length(env, 12, &ekadhipatya);
    for (uint32_t i = 0; i < 12; ++i) {
        napi_set_element(env, total, i, MakeUint32(env, s.total_points[i]));
        napi_set_element(env, trikona, i, MakeUint32(env, s.after_trikona[i]));
        napi_set_element(env, ekadhipatya, i, MakeUint32(env, s.after_ekadhipatya[i]));
    }
    SetNamed(env, obj, "totalPoints", total);
    SetNamed(env, obj, "afterTrikona", trikona);
    SetNamed(env, obj, "afterEkadhipatya", ekadhipatya);
    return obj;
}

napi_value WriteAshtakavargaResult(napi_env env, const DhruvAshtakavargaResult& a) {
    napi_value obj;
    napi_create_object(env, &obj);
    napi_value bavs;
    napi_create_array_with_length(env, DHRUV_SAPTA_GRAHA_COUNT, &bavs);
    for (uint32_t i = 0; i < DHRUV_SAPTA_GRAHA_COUNT; ++i) {
        napi_set_element(env, bavs, i, WriteBhinnaAshtakavarga(env, a.bavs[i]));
    }
    SetNamed(env, obj, "bavs", bavs);
    SetNamed(env, obj, "sav", WriteSarvaAshtakavarga(env, a.sav));
    return obj;
}

napi_value WriteAmshaEntry(napi_env env, const DhruvAmshaEntry& a) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "siderealLongitude", MakeDouble(env, a.sidereal_longitude));
    SetNamed(env, obj, "rashiIndex", MakeUint32(env, a.rashi_index));
    SetNamed(env, obj, "dmsDegrees", MakeUint32(env, a.dms_degrees));
    SetNamed(env, obj, "dmsMinutes", MakeUint32(env, a.dms_minutes));
    SetNamed(env, obj, "dmsSeconds", MakeDouble(env, a.dms_seconds));
    SetNamed(env, obj, "degreesInRashi", MakeDouble(env, a.degrees_in_rashi));
    return obj;
}

napi_value WriteAmshaChart(napi_env env, const DhruvAmshaChart& a) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "amshaCode", MakeUint32(env, a.amsha_code));
    SetNamed(env, obj, "variationCode", MakeUint32(env, a.variation_code));
    napi_value grahas;
    napi_create_array_with_length(env, DHRUV_GRAHA_COUNT, &grahas);
    for (uint32_t i = 0; i < DHRUV_GRAHA_COUNT; ++i) {
        napi_set_element(env, grahas, i, WriteAmshaEntry(env, a.grahas[i]));
    }
    SetNamed(env, obj, "grahas", grahas);
    SetNamed(env, obj, "lagna", WriteAmshaEntry(env, a.lagna));
    SetNamed(env, obj, "bhavaCuspsValid", MakeBool(env, a.bhava_cusps_valid != 0));
    SetNamed(env, obj, "arudhaPadasValid", MakeBool(env, a.arudha_padas_valid != 0));
    SetNamed(env, obj, "upagrahasValid", MakeBool(env, a.upagrahas_valid != 0));
    SetNamed(env, obj, "sphutasValid", MakeBool(env, a.sphutas_valid != 0));
    SetNamed(env, obj, "specialLagnasValid", MakeBool(env, a.special_lagnas_valid != 0));
    if (a.bhava_cusps_valid != 0) {
        napi_value bhava_cusps;
        napi_create_array_with_length(env, 12, &bhava_cusps);
        for (uint32_t i = 0; i < 12; ++i) {
            napi_set_element(env, bhava_cusps, i, WriteAmshaEntry(env, a.bhava_cusps[i]));
        }
        SetNamed(env, obj, "bhavaCusps", bhava_cusps);
    }
    if (a.arudha_padas_valid != 0) {
        napi_value arudha_padas;
        napi_create_array_with_length(env, 12, &arudha_padas);
        for (uint32_t i = 0; i < 12; ++i) {
            napi_set_element(env, arudha_padas, i, WriteAmshaEntry(env, a.arudha_padas[i]));
        }
        SetNamed(env, obj, "arudhaPadas", arudha_padas);
    }
    if (a.upagrahas_valid != 0) {
        napi_value upagrahas;
        napi_create_array_with_length(env, 11, &upagrahas);
        for (uint32_t i = 0; i < 11; ++i) {
            napi_set_element(env, upagrahas, i, WriteAmshaEntry(env, a.upagrahas[i]));
        }
        SetNamed(env, obj, "upagrahas", upagrahas);
    }
    if (a.sphutas_valid != 0) {
        napi_value sphutas;
        napi_create_array_with_length(env, 16, &sphutas);
        for (uint32_t i = 0; i < 16; ++i) {
            napi_set_element(env, sphutas, i, WriteAmshaEntry(env, a.sphutas[i]));
        }
        SetNamed(env, obj, "sphutas", sphutas);
    }
    if (a.special_lagnas_valid != 0) {
        napi_value special_lagnas;
        napi_create_array_with_length(env, 8, &special_lagnas);
        for (uint32_t i = 0; i < 8; ++i) {
            napi_set_element(env, special_lagnas, i, WriteAmshaEntry(env, a.special_lagnas[i]));
        }
        SetNamed(env, obj, "specialLagnas", special_lagnas);
    }
    return obj;
}

napi_value WriteGrahaDrishtiMatrix(napi_env env, const DhruvGrahaDrishtiMatrix& m) {
    napi_value matrix;
    napi_create_array_with_length(env, DHRUV_GRAHA_COUNT, &matrix);
    for (uint32_t i = 0; i < DHRUV_GRAHA_COUNT; ++i) {
        napi_value row;
        napi_create_array_with_length(env, DHRUV_GRAHA_COUNT, &row);
        for (uint32_t j = 0; j < DHRUV_GRAHA_COUNT; ++j) {
            napi_set_element(env, row, j, WriteDrishtiEntry(env, m.entries[i][j]));
        }
        napi_set_element(env, matrix, i, row);
    }
    return matrix;
}

napi_value WriteDrishtiResult(napi_env env, const DhruvDrishtiResult& d) {
    napi_value obj;
    napi_create_object(env, &obj);

    napi_value grahaToGraha;
    napi_create_array_with_length(env, DHRUV_GRAHA_COUNT, &grahaToGraha);
    napi_value grahaToBhava;
    napi_create_array_with_length(env, DHRUV_GRAHA_COUNT, &grahaToBhava);
    napi_value grahaToLagna;
    napi_create_array_with_length(env, DHRUV_GRAHA_COUNT, &grahaToLagna);
    napi_value grahaToBindus;
    napi_create_array_with_length(env, DHRUV_GRAHA_COUNT, &grahaToBindus);

    for (uint32_t i = 0; i < DHRUV_GRAHA_COUNT; ++i) {
        napi_value rowGG;
        napi_create_array_with_length(env, DHRUV_GRAHA_COUNT, &rowGG);
        for (uint32_t j = 0; j < DHRUV_GRAHA_COUNT; ++j) {
            napi_set_element(env, rowGG, j, WriteDrishtiEntry(env, d.graha_to_graha[i][j]));
        }
        napi_set_element(env, grahaToGraha, i, rowGG);

        napi_value rowGB;
        napi_create_array_with_length(env, 12, &rowGB);
        for (uint32_t j = 0; j < 12; ++j) {
            napi_set_element(env, rowGB, j, WriteDrishtiEntry(env, d.graha_to_bhava[i][j]));
        }
        napi_set_element(env, grahaToBhava, i, rowGB);

        napi_set_element(env, grahaToLagna, i, WriteDrishtiEntry(env, d.graha_to_lagna[i]));

        napi_value rowBindus;
        napi_create_array_with_length(env, 19, &rowBindus);
        for (uint32_t j = 0; j < 19; ++j) {
            napi_set_element(env, rowBindus, j, WriteDrishtiEntry(env, d.graha_to_bindus[i][j]));
        }
        napi_set_element(env, grahaToBindus, i, rowBindus);
    }

    SetNamed(env, obj, "grahaToGraha", grahaToGraha);
    SetNamed(env, obj, "grahaToBhava", grahaToBhava);
    SetNamed(env, obj, "grahaToLagna", grahaToLagna);
    SetNamed(env, obj, "grahaToBindus", grahaToBindus);
    return obj;
}

napi_value WriteGrahaPositions(napi_env env, const DhruvGrahaPositions& p) {
    napi_value obj;
    napi_create_object(env, &obj);
    napi_value grahas;
    napi_create_array_with_length(env, DHRUV_GRAHA_COUNT, &grahas);
    for (uint32_t i = 0; i < DHRUV_GRAHA_COUNT; ++i) {
        napi_set_element(env, grahas, i, WriteGrahaEntry(env, p.grahas[i]));
    }
    SetNamed(env, obj, "grahas", grahas);
    SetNamed(env, obj, "lagna", WriteGrahaEntry(env, p.lagna));
    napi_value outer;
    napi_create_array_with_length(env, 3, &outer);
    for (uint32_t i = 0; i < 3; ++i) {
        napi_set_element(env, outer, i, WriteGrahaEntry(env, p.outer_planets[i]));
    }
    SetNamed(env, obj, "outerPlanets", outer);
    return obj;
}

napi_value WriteBindusResult(napi_env env, const DhruvBindusResult& b) {
    napi_value obj;
    napi_create_object(env, &obj);
    napi_value arudhas;
    napi_create_array_with_length(env, 12, &arudhas);
    for (uint32_t i = 0; i < 12; ++i) {
        napi_set_element(env, arudhas, i, WriteGrahaEntry(env, b.arudha_padas[i]));
    }
    SetNamed(env, obj, "arudhaPadas", arudhas);
    SetNamed(env, obj, "bhriguBindu", WriteGrahaEntry(env, b.bhrigu_bindu));
    SetNamed(env, obj, "pranapadaLagna", WriteGrahaEntry(env, b.pranapada_lagna));
    SetNamed(env, obj, "gulika", WriteGrahaEntry(env, b.gulika));
    SetNamed(env, obj, "maandi", WriteGrahaEntry(env, b.maandi));
    SetNamed(env, obj, "horaLagna", WriteGrahaEntry(env, b.hora_lagna));
    SetNamed(env, obj, "ghatiLagna", WriteGrahaEntry(env, b.ghati_lagna));
    SetNamed(env, obj, "sreeLagna", WriteGrahaEntry(env, b.sree_lagna));
    return obj;
}

napi_value WriteConjunctionEvent(napi_env env, const DhruvConjunctionEvent& ev) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "jdTdb", MakeDouble(env, ev.jd_tdb));
    SetNamed(env, obj, "actualSeparationDeg", MakeDouble(env, ev.actual_separation_deg));
    SetNamed(env, obj, "body1LongitudeDeg", MakeDouble(env, ev.body1_longitude_deg));
    SetNamed(env, obj, "body2LongitudeDeg", MakeDouble(env, ev.body2_longitude_deg));
    SetNamed(env, obj, "body1LatitudeDeg", MakeDouble(env, ev.body1_latitude_deg));
    SetNamed(env, obj, "body2LatitudeDeg", MakeDouble(env, ev.body2_latitude_deg));
    SetNamed(env, obj, "body1Code", MakeInt32(env, ev.body1_code));
    SetNamed(env, obj, "body2Code", MakeInt32(env, ev.body2_code));
    return obj;
}

napi_value WriteSankrantiEvent(napi_env env, const DhruvSankrantiEvent& ev) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "utc", WriteUtcTime(env, ev.utc));
    SetNamed(env, obj, "rashiIndex", MakeInt32(env, ev.rashi_index));
    SetNamed(env, obj, "sunSiderealLongitudeDeg", MakeDouble(env, ev.sun_sidereal_longitude_deg));
    SetNamed(env, obj, "sunTropicalLongitudeDeg", MakeDouble(env, ev.sun_tropical_longitude_deg));
    return obj;
}

napi_value WriteStationaryEvent(napi_env env, const DhruvStationaryEvent& ev) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "jdTdb", MakeDouble(env, ev.jd_tdb));
    SetNamed(env, obj, "bodyCode", MakeInt32(env, ev.body_code));
    SetNamed(env, obj, "longitudeDeg", MakeDouble(env, ev.longitude_deg));
    SetNamed(env, obj, "latitudeDeg", MakeDouble(env, ev.latitude_deg));
    SetNamed(env, obj, "stationType", MakeInt32(env, ev.station_type));
    return obj;
}

napi_value WriteMaxSpeedEvent(napi_env env, const DhruvMaxSpeedEvent& ev) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "jdTdb", MakeDouble(env, ev.jd_tdb));
    SetNamed(env, obj, "bodyCode", MakeInt32(env, ev.body_code));
    SetNamed(env, obj, "longitudeDeg", MakeDouble(env, ev.longitude_deg));
    SetNamed(env, obj, "latitudeDeg", MakeDouble(env, ev.latitude_deg));
    SetNamed(env, obj, "speedDegPerDay", MakeDouble(env, ev.speed_deg_per_day));
    SetNamed(env, obj, "speedType", MakeInt32(env, ev.speed_type));
    return obj;
}

napi_value WriteChandraGrahanResult(napi_env env, const DhruvChandraGrahanResult& g) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "grahanType", MakeInt32(env, g.grahan_type));
    SetNamed(env, obj, "magnitude", MakeDouble(env, g.magnitude));
    SetNamed(env, obj, "penumbralMagnitude", MakeDouble(env, g.penumbral_magnitude));
    SetNamed(env, obj, "greatestGrahanJd", MakeDouble(env, g.greatest_grahan_jd));
    SetNamed(env, obj, "p1Jd", MakeDouble(env, g.p1_jd));
    SetNamed(env, obj, "u1Jd", MakeDouble(env, g.u1_jd));
    SetNamed(env, obj, "u2Jd", MakeDouble(env, g.u2_jd));
    SetNamed(env, obj, "u3Jd", MakeDouble(env, g.u3_jd));
    SetNamed(env, obj, "u4Jd", MakeDouble(env, g.u4_jd));
    SetNamed(env, obj, "p4Jd", MakeDouble(env, g.p4_jd));
    SetNamed(env, obj, "moonEclipticLatDeg", MakeDouble(env, g.moon_ecliptic_lat_deg));
    SetNamed(env, obj, "angularSeparationDeg", MakeDouble(env, g.angular_separation_deg));
    return obj;
}

napi_value WriteSuryaGrahanResult(napi_env env, const DhruvSuryaGrahanResult& g) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "grahanType", MakeInt32(env, g.grahan_type));
    SetNamed(env, obj, "magnitude", MakeDouble(env, g.magnitude));
    SetNamed(env, obj, "greatestGrahanJd", MakeDouble(env, g.greatest_grahan_jd));
    SetNamed(env, obj, "c1Jd", MakeDouble(env, g.c1_jd));
    SetNamed(env, obj, "c2Jd", MakeDouble(env, g.c2_jd));
    SetNamed(env, obj, "c3Jd", MakeDouble(env, g.c3_jd));
    SetNamed(env, obj, "c4Jd", MakeDouble(env, g.c4_jd));
    SetNamed(env, obj, "moonEclipticLatDeg", MakeDouble(env, g.moon_ecliptic_lat_deg));
    SetNamed(env, obj, "angularSeparationDeg", MakeDouble(env, g.angular_separation_deg));
    return obj;
}

napi_value WriteRiseSetResult(napi_env env, const DhruvRiseSetResult& r) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "resultType", MakeInt32(env, r.result_type));
    SetNamed(env, obj, "eventCode", MakeInt32(env, r.event_code));
    SetNamed(env, obj, "jdTdb", MakeDouble(env, r.jd_tdb));
    return obj;
}

napi_value WriteRiseSetResultUtc(napi_env env, const DhruvRiseSetResultUtc& r) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "resultType", MakeInt32(env, r.result_type));
    SetNamed(env, obj, "eventCode", MakeInt32(env, r.event_code));
    SetNamed(env, obj, "utc", WriteUtcTime(env, r.utc));
    return obj;
}

napi_value WriteBhavaResult(napi_env env, const DhruvBhavaResult& b) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "lagnaDeg", MakeDouble(env, b.lagna_deg));
    SetNamed(env, obj, "mcDeg", MakeDouble(env, b.mc_deg));
    napi_value arr;
    napi_create_array_with_length(env, 12, &arr);
    for (uint32_t i = 0; i < 12; ++i) {
        napi_value x;
        napi_create_object(env, &x);
        SetNamed(env, x, "number", MakeUint32(env, b.bhavas[i].number));
        SetNamed(env, x, "cuspDeg", MakeDouble(env, b.bhavas[i].cusp_deg));
        SetNamed(env, x, "startDeg", MakeDouble(env, b.bhavas[i].start_deg));
        SetNamed(env, x, "endDeg", MakeDouble(env, b.bhavas[i].end_deg));
        napi_set_element(env, arr, i, x);
    }
    SetNamed(env, obj, "bhavas", arr);
    return obj;
}

napi_value WriteRashiInfo(napi_env env, const DhruvRashiInfo& r) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "rashiIndex", MakeUint32(env, r.rashi_index));
    SetNamed(env, obj, "degreesInRashi", MakeDouble(env, r.degrees_in_rashi));
    napi_value dms;
    napi_create_object(env, &dms);
    SetNamed(env, dms, "degrees", MakeUint32(env, r.dms.degrees));
    SetNamed(env, dms, "minutes", MakeUint32(env, r.dms.minutes));
    SetNamed(env, dms, "seconds", MakeDouble(env, r.dms.seconds));
    SetNamed(env, obj, "dms", dms);
    return obj;
}

napi_value WriteNakshatraInfo(napi_env env, const DhruvNakshatraInfo& n) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "nakshatraIndex", MakeUint32(env, n.nakshatra_index));
    SetNamed(env, obj, "pada", MakeUint32(env, n.pada));
    SetNamed(env, obj, "degreesInNakshatra", MakeDouble(env, n.degrees_in_nakshatra));
    SetNamed(env, obj, "degreesInPada", MakeDouble(env, n.degrees_in_pada));
    return obj;
}

napi_value WriteNakshatra28Info(napi_env env, const DhruvNakshatra28Info& n) {
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "nakshatraIndex", MakeUint32(env, n.nakshatra_index));
    SetNamed(env, obj, "pada", MakeUint32(env, n.pada));
    SetNamed(env, obj, "degreesInNakshatra", MakeDouble(env, n.degrees_in_nakshatra));
    return obj;
}

napi_value ApiVersion(napi_env env, napi_callback_info info) {
    (void)info;
    return MakeUint32(env, dhruv_api_version());
}

napi_value ConfigClearActive(napi_env env, napi_callback_info info) {
    (void)info;
    return MakeInt32(env, dhruv_config_clear_active());
}

napi_value ConfigLoad(napi_env env, napi_callback_info info) {
    size_t argc = 2;
    napi_value args[2];
    NAPI_RETURN_IF_FAILED(env, napi_get_cb_info(env, info, &argc, args, nullptr, nullptr));

    if (argc < 2) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    bool is_null = false;
    napi_valuetype t;
    napi_typeof(env, args[0], &t);
    is_null = (t == napi_null || t == napi_undefined);

    uint32_t defaults_mode = 0;
    if (!GetUint32(env, args[1], &defaults_mode)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    const uint8_t* path_ptr = nullptr;
    std::string path;
    uint32_t path_len = 0;
    if (!is_null) {
        if (!GetString(env, args[0], &path)) {
            return MakeStatusResult(env, STATUS_INVALID_INPUT);
        }
        path_ptr = reinterpret_cast<const uint8_t*>(path.data());
        path_len = static_cast<uint32_t>(path.size());
    }

    DhruvConfigHandle* handle = nullptr;
    int32_t status = dhruv_config_load(path_ptr, path_len, &handle);

    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK && handle != nullptr) {
        SetNamed(env, out, "handle", MakeExternalPtr(env, handle));
    } else {
        napi_value nullv;
        napi_get_null(env, &nullv);
        SetNamed(env, out, "handle", nullv);
    }
    return out;
}

napi_value ConfigFree(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value args[1];
    NAPI_RETURN_IF_FAILED(env, napi_get_cb_info(env, info, &argc, args, nullptr, nullptr));
    if (argc < 1) {
        return MakeInt32(env, STATUS_INVALID_INPUT);
    }

    void* ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &ptr)) {
        return MakeInt32(env, STATUS_INVALID_INPUT);
    }

    return MakeInt32(env, dhruv_config_free(static_cast<DhruvConfigHandle*>(ptr)));
}

napi_value EngineNew(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value args[1];
    NAPI_RETURN_IF_FAILED(env, napi_get_cb_info(env, info, &argc, args, nullptr, nullptr));
    if (argc < 1) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvEngineConfig cfg{};
    cfg.cache_capacity = 256;
    cfg.strict_validation = 1;

    napi_value spk_paths_val;
    if (!GetNamedProperty(env, args[0], "spkPaths", &spk_paths_val)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    bool is_array = false;
    napi_is_array(env, spk_paths_val, &is_array);
    if (!is_array) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    uint32_t spk_count = 0;
    napi_get_array_length(env, spk_paths_val, &spk_count);
    if (spk_count > DHRUV_MAX_SPK_PATHS) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    cfg.spk_path_count = spk_count;
    for (uint32_t i = 0; i < spk_count; ++i) {
        napi_value item;
        napi_get_element(env, spk_paths_val, i, &item);
        std::string s;
        if (!GetString(env, item, &s) || s.size() >= DHRUV_PATH_CAPACITY) {
            return MakeStatusResult(env, STATUS_INVALID_INPUT);
        }
        std::memcpy(cfg.spk_paths_utf8[i], s.data(), s.size());
        cfg.spk_paths_utf8[i][s.size()] = '\0';
    }

    napi_value lsk_val;
    bool has_lsk = false;
    if (!GetOptionalNamedProperty(env, args[0], "lskPath", &lsk_val, &has_lsk)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }
    if (has_lsk) {
        napi_valuetype tt;
        napi_typeof(env, lsk_val, &tt);
        if (tt != napi_null && tt != napi_undefined) {
            std::string s;
            if (!GetString(env, lsk_val, &s) || s.size() >= DHRUV_PATH_CAPACITY) {
                return MakeStatusResult(env, STATUS_INVALID_INPUT);
            }
            std::memcpy(cfg.lsk_path_utf8, s.data(), s.size());
            cfg.lsk_path_utf8[s.size()] = '\0';
        }
    }

    napi_value cache_val;
    bool has_cache = false;
    if (GetOptionalNamedProperty(env, args[0], "cacheCapacity", &cache_val, &has_cache) && has_cache) {
        uint64_t cap = 0;
        if (GetUint64(env, cache_val, &cap)) {
            cfg.cache_capacity = cap;
        } else {
            double dcap = 0;
            if (!GetDouble(env, cache_val, &dcap)) {
                return MakeStatusResult(env, STATUS_INVALID_INPUT);
            }
            cfg.cache_capacity = static_cast<uint64_t>(std::max(0.0, dcap));
        }
    }

    napi_value strict_val;
    bool has_strict = false;
    if (GetOptionalNamedProperty(env, args[0], "strictValidation", &strict_val, &has_strict) && has_strict) {
        bool strict = true;
        if (!GetBool(env, strict_val, &strict)) {
            return MakeStatusResult(env, STATUS_INVALID_INPUT);
        }
        cfg.strict_validation = strict ? 1 : 0;
    }

    DhruvEngineHandle* engine = nullptr;
    int32_t status = dhruv_engine_new(&cfg, &engine);

    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK && engine != nullptr) {
        SetNamed(env, out, "handle", MakeExternalPtr(env, engine));
    } else {
        napi_value nullv;
        napi_get_null(env, &nullv);
        SetNamed(env, out, "handle", nullv);
    }
    return out;
}

napi_value EngineFree(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value args[1];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 1) {
        return MakeInt32(env, STATUS_INVALID_INPUT);
    }

    void* ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &ptr)) {
        return MakeInt32(env, STATUS_INVALID_INPUT);
    }

    return MakeInt32(env, dhruv_engine_free(static_cast<DhruvEngineHandle*>(ptr)));
}

napi_value LskLoad(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value args[1];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 1) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    std::string path;
    if (!GetString(env, args[0], &path)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvLskHandle* handle = nullptr;
    int32_t status = dhruv_lsk_load(path.c_str(), &handle);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK && handle != nullptr) {
        SetNamed(env, out, "handle", MakeExternalPtr(env, handle));
    } else {
        napi_value nullv;
        napi_get_null(env, &nullv);
        SetNamed(env, out, "handle", nullv);
    }
    return out;
}

napi_value LskFree(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value args[1];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 1) {
        return MakeInt32(env, STATUS_INVALID_INPUT);
    }

    void* ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &ptr)) {
        return MakeInt32(env, STATUS_INVALID_INPUT);
    }

    return MakeInt32(env, dhruv_lsk_free(static_cast<DhruvLskHandle*>(ptr)));
}

napi_value EopLoad(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value args[1];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 1) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    std::string path;
    if (!GetString(env, args[0], &path)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvEopHandle* handle = nullptr;
    int32_t status = dhruv_eop_load(path.c_str(), &handle);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK && handle != nullptr) {
        SetNamed(env, out, "handle", MakeExternalPtr(env, handle));
    } else {
        napi_value nullv;
        napi_get_null(env, &nullv);
        SetNamed(env, out, "handle", nullv);
    }
    return out;
}

napi_value EopFree(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value args[1];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 1) {
        return MakeInt32(env, STATUS_INVALID_INPUT);
    }

    void* ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &ptr)) {
        return MakeInt32(env, STATUS_INVALID_INPUT);
    }

    return MakeInt32(env, dhruv_eop_free(static_cast<DhruvEopHandle*>(ptr)));
}

napi_value EngineQuery(napi_env env, napi_callback_info info) {
    size_t argc = 2;
    napi_value args[2];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 2) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    void* ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &ptr)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvQueryRequest request{};
    if (!ReadQueryRequest(env, args[1], &request)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvQueryResult out_result{};
    int32_t status =
        dhruv_engine_query_request(static_cast<const DhruvEngineHandle*>(ptr), &request, &out_result);

    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "result", WriteQueryResult(env, out_result, request.output_mode));
    }
    return out;
}

napi_value QueryOnce(napi_env env, napi_callback_info info) {
    size_t argc = 2;
    napi_value args[2];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 2) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvEngineConfig cfg{};
    cfg.cache_capacity = 256;
    cfg.strict_validation = 1;

    napi_value spk_paths_val;
    if (!GetNamedProperty(env, args[0], "spkPaths", &spk_paths_val)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    bool is_array = false;
    napi_is_array(env, spk_paths_val, &is_array);
    if (!is_array) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    uint32_t spk_count = 0;
    napi_get_array_length(env, spk_paths_val, &spk_count);
    if (spk_count > DHRUV_MAX_SPK_PATHS) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    cfg.spk_path_count = spk_count;
    for (uint32_t i = 0; i < spk_count; ++i) {
        napi_value item;
        std::string s;
        napi_get_element(env, spk_paths_val, i, &item);
        if (!GetString(env, item, &s) || s.size() >= DHRUV_PATH_CAPACITY) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        std::memcpy(cfg.spk_paths_utf8[i], s.data(), s.size());
        cfg.spk_paths_utf8[i][s.size()] = '\0';
    }
    napi_value lsk_val;
    bool has_lsk = false;
    if (!GetOptionalNamedProperty(env, args[0], "lskPath", &lsk_val, &has_lsk)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (has_lsk) {
        napi_valuetype tt;
        napi_typeof(env, lsk_val, &tt);
        if (tt != napi_null && tt != napi_undefined) {
            std::string s;
            if (!GetString(env, lsk_val, &s) || s.size() >= DHRUV_PATH_CAPACITY) return MakeStatusResult(env, STATUS_INVALID_INPUT);
            std::memcpy(cfg.lsk_path_utf8, s.data(), s.size());
            cfg.lsk_path_utf8[s.size()] = '\0';
        }
    }
    napi_value cache_val;
    bool has_cache = false;
    if (GetOptionalNamedProperty(env, args[0], "cacheCapacity", &cache_val, &has_cache) && has_cache) {
        uint64_t cap = 0;
        if (GetUint64(env, cache_val, &cap)) {
            cfg.cache_capacity = cap;
        } else {
            double dcap = 0;
            if (!GetDouble(env, cache_val, &dcap)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
            cfg.cache_capacity = static_cast<uint64_t>(std::max(0.0, dcap));
        }
    }
    napi_value strict_val;
    bool has_strict = false;
    if (GetOptionalNamedProperty(env, args[0], "strictValidation", &strict_val, &has_strict) && has_strict) {
        bool strict = true;
        if (!GetBool(env, strict_val, &strict)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        cfg.strict_validation = strict ? 1 : 0;
    }

    DhruvQuery q{};
    napi_value v;
    if (!GetNamedProperty(env, args[1], "target", &v) || !GetInt32(env, v, &q.target)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "observer", &v) || !GetInt32(env, v, &q.observer)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "frame", &v) || !GetInt32(env, v, &q.frame)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "epochTdbJd", &v) || !GetDouble(env, v, &q.epoch_tdb_jd)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvStateVector out_vec{};
    int32_t status = dhruv_query_once(&cfg, &q, &out_vec);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "state", WriteStateVector(env, out_vec));
    return out;
}

napi_value CartesianToSpherical(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value args[1];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 1) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double pos[3]{};
    if (!ReadDoubleArrayFixed(env, args[0], pos, 3)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvSphericalCoords out_coords{};
    int32_t status = dhruv_cartesian_to_spherical(pos, &out_coords);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        napi_value c;
        napi_create_object(env, &c);
        SetNamed(env, c, "lonDeg", MakeDouble(env, out_coords.lon_deg));
        SetNamed(env, c, "latDeg", MakeDouble(env, out_coords.lat_deg));
        SetNamed(env, c, "distanceKm", MakeDouble(env, out_coords.distance_km));
        SetNamed(env, out, "coords", c);
    }
    return out;
}

napi_value UtcToTdbJd(napi_env env, napi_callback_info info) {
    size_t argc = 2;
    napi_value args[2];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 2) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    void* ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &ptr)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvUtcTime utc{};
    if (!ReadUtcTime(env, args[1], &utc)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    double jd = 0.0;
    int32_t status = dhruv_utc_to_tdb_jd(
        static_cast<const DhruvLskHandle*>(ptr),
        utc.year,
        utc.month,
        utc.day,
        utc.hour,
        utc.minute,
        utc.second,
        &jd);

    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "jdTdb", MakeDouble(env, jd));
    }
    return out;
}

napi_value JdTdbToUtc(napi_env env, napi_callback_info info) {
    size_t argc = 2;
    napi_value args[2];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 2) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    void* ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &ptr)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    double jd = 0.0;
    if (!GetDouble(env, args[1], &jd)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvUtcTime utc{};
    int32_t status = dhruv_jd_tdb_to_utc(static_cast<const DhruvLskHandle*>(ptr), jd, &utc);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "utc", WriteUtcTime(env, utc));
    }
    return out;
}

napi_value NutationIau2000b(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value args[1];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 1) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    double jd = 0.0;
    if (!GetDouble(env, args[0], &jd)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    double dpsi = 0.0;
    double deps = 0.0;
    int32_t status = dhruv_nutation_iau2000b(jd, &dpsi, &deps);

    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "dpsi", MakeDouble(env, dpsi));
        SetNamed(env, out, "deps", MakeDouble(env, deps));
    }
    return out;
}

napi_value NutationIau2000bUtc(napi_env env, napi_callback_info info) {
    size_t argc = 2;
    napi_value args[2];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 2) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* lsk_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &lsk_ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvUtcTime utc{};
    if (!ReadUtcTime(env, args[1], &utc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double dpsi = 0.0;
    double deps = 0.0;
    int32_t status = dhruv_nutation_iau2000b_utc(static_cast<const DhruvLskHandle*>(lsk_ptr), &utc, &dpsi, &deps);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "dpsi", MakeDouble(env, dpsi));
        SetNamed(env, out, "deps", MakeDouble(env, deps));
    }
    return out;
}

napi_value ApproximateLocalNoonJd(napi_env env, napi_callback_info info) {
    size_t argc = 2;
    napi_value args[2];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 2) return MakeDouble(env, 0.0);
    double jd = 0.0;
    double lon = 0.0;
    if (!GetDouble(env, args[0], &jd) || !GetDouble(env, args[1], &lon)) return MakeDouble(env, 0.0);
    return MakeDouble(env, dhruv_approximate_local_noon_jd(jd, lon));
}

napi_value AyanamshaSystemCount(napi_env env, napi_callback_info info) {
    (void)info;
    return MakeUint32(env, dhruv_ayanamsha_system_count());
}

napi_value ReferencePlaneDefault(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value args[1];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 1) return MakeInt32(env, STATUS_INVALID_INPUT);
    int32_t system_code = 0;
    if (!GetInt32(env, args[0], &system_code)) return MakeInt32(env, STATUS_INVALID_INPUT);
    return MakeInt32(env, dhruv_reference_plane_default(system_code));
}

napi_value AyanamshaComputeEx(napi_env env, napi_callback_info info) {
    size_t argc = 3;
    napi_value args[3];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 3) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    void* lsk_ptr = nullptr;
    void* eop_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &lsk_ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!ReadExternalPtr(env, args[2], &eop_ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvAyanamshaComputeRequest req{};
    napi_value v;
    if (!GetNamedProperty(env, args[1], "systemCode", &v) || !GetInt32(env, v, &req.system_code)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "mode", &v) || !GetInt32(env, v, &req.mode)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "timeKind", &v) || !GetInt32(env, v, &req.time_kind)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "jdTdb", &v) || !GetDouble(env, v, &req.jd_tdb)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "utc", &v) || !ReadUtcTime(env, v, &req.utc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "useNutation", &v)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    bool use_nutation = false;
    if (!GetBool(env, v, &use_nutation)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    req.use_nutation = use_nutation ? 1 : 0;
    bool has_dpsi = false;
    if (!GetOptionalNamedProperty(env, args[1], "deltaPsiArcsec", &v, &has_dpsi)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    req.delta_psi_arcsec = 0.0;
    if (has_dpsi && !GetDouble(env, v, &req.delta_psi_arcsec)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    double out_val = 0.0;
    int32_t status = dhruv_ayanamsha_compute_ex(
        static_cast<const DhruvLskHandle*>(lsk_ptr),
        &req,
        static_cast<const DhruvEopHandle*>(eop_ptr),
        &out_val);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "ayanamshaDeg", MakeDouble(env, out_val));
    return out;
}

napi_value LunarNodeCount(napi_env env, napi_callback_info info) {
    (void)info;
    return MakeUint32(env, dhruv_lunar_node_count());
}

napi_value LunarNodeDeg(napi_env env, napi_callback_info info) {
    size_t argc = 3;
    napi_value args[3];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 3) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    int32_t node_code = 0;
    int32_t mode_code = 0;
    double jd = 0.0;
    if (!GetInt32(env, args[0], &node_code) || !GetInt32(env, args[1], &mode_code) || !GetDouble(env, args[2], &jd)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double out_val = 0.0;
    int32_t status = dhruv_lunar_node_deg(node_code, mode_code, jd, &out_val);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "longitudeDeg", MakeDouble(env, out_val));
    return out;
}

napi_value LunarNodeDegWithEngine(napi_env env, napi_callback_info info) {
    size_t argc = 4;
    napi_value args[4];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 4) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* e_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &e_ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    int32_t node_code = 0;
    int32_t mode_code = 0;
    double jd = 0.0;
    if (!GetInt32(env, args[1], &node_code) || !GetInt32(env, args[2], &mode_code) || !GetDouble(env, args[3], &jd)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double out_val = 0.0;
    int32_t status = dhruv_lunar_node_deg_with_engine(static_cast<const DhruvEngineHandle*>(e_ptr), node_code, mode_code, jd, &out_val);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "longitudeDeg", MakeDouble(env, out_val));
    return out;
}

napi_value LunarNodeDegUtc(napi_env env, napi_callback_info info) {
    size_t argc = 4;
    napi_value args[4];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 4) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* lsk_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &lsk_ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    int32_t node_code = 0;
    int32_t mode_code = 0;
    if (!GetInt32(env, args[1], &node_code) || !GetInt32(env, args[2], &mode_code)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvUtcTime utc{};
    if (!ReadUtcTime(env, args[3], &utc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double out_val = 0.0;
    int32_t status = dhruv_lunar_node_deg_utc(static_cast<const DhruvLskHandle*>(lsk_ptr), node_code, mode_code, &utc, &out_val);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "longitudeDeg", MakeDouble(env, out_val));
    return out;
}

napi_value LunarNodeDegUtcWithEngine(napi_env env, napi_callback_info info) {
    size_t argc = 5;
    napi_value args[5];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 5) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* e_ptr = nullptr;
    void* lsk_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &lsk_ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    int32_t node_code = 0;
    int32_t mode_code = 0;
    if (!GetInt32(env, args[2], &node_code) || !GetInt32(env, args[3], &mode_code)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvUtcTime utc{};
    if (!ReadUtcTime(env, args[4], &utc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double out_val = 0.0;
    int32_t status = dhruv_lunar_node_deg_utc_with_engine(
        static_cast<const DhruvEngineHandle*>(e_ptr),
        static_cast<const DhruvLskHandle*>(lsk_ptr),
        node_code,
        mode_code,
        &utc,
        &out_val);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "longitudeDeg", MakeDouble(env, out_val));
    return out;
}

napi_value LunarNodeComputeEx(napi_env env, napi_callback_info info) {
    size_t argc = 3;
    napi_value args[3];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 3) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* lsk_ptr = nullptr;
    void* eop_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &lsk_ptr) || !ReadExternalPtr(env, args[1], &eop_ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvLunarNodeRequest req{};
    napi_value v;
    if (!GetNamedProperty(env, args[2], "nodeCode", &v) || !GetInt32(env, v, &req.node_code)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[2], "modeCode", &v) || !GetInt32(env, v, &req.mode_code)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[2], "backend", &v) || !GetInt32(env, v, &req.backend)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[2], "timeKind", &v) || !GetInt32(env, v, &req.time_kind)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[2], "jdTdb", &v) || !GetDouble(env, v, &req.jd_tdb)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[2], "utc", &v) || !ReadUtcTime(env, v, &req.utc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double out_val = 0.0;
    int32_t status = dhruv_lunar_node_compute_ex(
        static_cast<const DhruvLskHandle*>(lsk_ptr),
        static_cast<const DhruvEopHandle*>(eop_ptr),
        &req,
        &out_val);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "longitudeDeg", MakeDouble(env, out_val));
    return out;
}

napi_value RashiCount(napi_env env, napi_callback_info info) {
    (void)info;
    return MakeUint32(env, dhruv_rashi_count());
}

napi_value NakshatraCount(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value args[1];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 1) return MakeUint32(env, 0);
    uint32_t scheme = 0;
    if (!GetUint32(env, args[0], &scheme)) return MakeUint32(env, 0);
    return MakeUint32(env, dhruv_nakshatra_count(scheme));
}

napi_value RashiFromLongitude(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value args[1];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 1) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double lon = 0.0;
    if (!GetDouble(env, args[0], &lon)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvRashiInfo info_out{};
    int32_t status = dhruv_rashi_from_longitude(lon, &info_out);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "rashi", WriteRashiInfo(env, info_out));
    return out;
}

napi_value NakshatraFromLongitude(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value args[1];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 1) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double lon = 0.0;
    if (!GetDouble(env, args[0], &lon)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvNakshatraInfo info_out{};
    int32_t status = dhruv_nakshatra_from_longitude(lon, &info_out);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "nakshatra", WriteNakshatraInfo(env, info_out));
    return out;
}

napi_value Nakshatra28FromLongitude(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value args[1];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 1) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double lon = 0.0;
    if (!GetDouble(env, args[0], &lon)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvNakshatra28Info info_out{};
    int32_t status = dhruv_nakshatra28_from_longitude(lon, &info_out);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "nakshatra28", WriteNakshatra28Info(env, info_out));
    return out;
}

napi_value RashiFromTropical(napi_env env, napi_callback_info info) {
    size_t argc = 4;
    napi_value args[4];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 4) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double lon = 0.0;
    uint32_t ay = 0;
    double jd = 0.0;
    bool use_nut = false;
    if (!GetDouble(env, args[0], &lon) || !GetUint32(env, args[1], &ay) || !GetDouble(env, args[2], &jd) || !GetBool(env, args[3], &use_nut)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvRashiInfo out_info{};
    int32_t status = dhruv_rashi_from_tropical(lon, ay, jd, use_nut ? 1 : 0, &out_info);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "rashi", WriteRashiInfo(env, out_info));
    return out;
}

napi_value NakshatraFromTropical(napi_env env, napi_callback_info info) {
    size_t argc = 4;
    napi_value args[4];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 4) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double lon = 0.0;
    uint32_t ay = 0;
    double jd = 0.0;
    bool use_nut = false;
    if (!GetDouble(env, args[0], &lon) || !GetUint32(env, args[1], &ay) || !GetDouble(env, args[2], &jd) || !GetBool(env, args[3], &use_nut)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvNakshatraInfo out_info{};
    int32_t status = dhruv_nakshatra_from_tropical(lon, ay, jd, use_nut ? 1 : 0, &out_info);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "nakshatra", WriteNakshatraInfo(env, out_info));
    return out;
}

napi_value Nakshatra28FromTropical(napi_env env, napi_callback_info info) {
    size_t argc = 4;
    napi_value args[4];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 4) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double lon = 0.0;
    uint32_t ay = 0;
    double jd = 0.0;
    bool use_nut = false;
    if (!GetDouble(env, args[0], &lon) || !GetUint32(env, args[1], &ay) || !GetDouble(env, args[2], &jd) || !GetBool(env, args[3], &use_nut)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvNakshatra28Info out_info{};
    int32_t status = dhruv_nakshatra28_from_tropical(lon, ay, jd, use_nut ? 1 : 0, &out_info);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "nakshatra28", WriteNakshatra28Info(env, out_info));
    return out;
}

napi_value RashiFromTropicalUtc(napi_env env, napi_callback_info info) {
    size_t argc = 5;
    napi_value args[5];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 5) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* lsk_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &lsk_ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double lon = 0.0;
    uint32_t ay = 0;
    if (!GetDouble(env, args[1], &lon) || !GetUint32(env, args[2], &ay)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvUtcTime utc{};
    if (!ReadUtcTime(env, args[3], &utc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    bool use_nut = false;
    if (!GetBool(env, args[4], &use_nut)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvRashiInfo out_info{};
    int32_t status = dhruv_rashi_from_tropical_utc(static_cast<const DhruvLskHandle*>(lsk_ptr), lon, ay, &utc, use_nut ? 1 : 0, &out_info);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "rashi", WriteRashiInfo(env, out_info));
    return out;
}

napi_value NakshatraFromTropicalUtc(napi_env env, napi_callback_info info) {
    size_t argc = 5;
    napi_value args[5];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 5) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* lsk_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &lsk_ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double lon = 0.0;
    uint32_t ay = 0;
    if (!GetDouble(env, args[1], &lon) || !GetUint32(env, args[2], &ay)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvUtcTime utc{};
    if (!ReadUtcTime(env, args[3], &utc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    bool use_nut = false;
    if (!GetBool(env, args[4], &use_nut)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvNakshatraInfo out_info{};
    int32_t status = dhruv_nakshatra_from_tropical_utc(static_cast<const DhruvLskHandle*>(lsk_ptr), lon, ay, &utc, use_nut ? 1 : 0, &out_info);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "nakshatra", WriteNakshatraInfo(env, out_info));
    return out;
}

napi_value Nakshatra28FromTropicalUtc(napi_env env, napi_callback_info info) {
    size_t argc = 5;
    napi_value args[5];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 5) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* lsk_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &lsk_ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double lon = 0.0;
    uint32_t ay = 0;
    if (!GetDouble(env, args[1], &lon) || !GetUint32(env, args[2], &ay)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvUtcTime utc{};
    if (!ReadUtcTime(env, args[3], &utc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    bool use_nut = false;
    if (!GetBool(env, args[4], &use_nut)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvNakshatra28Info out_info{};
    int32_t status = dhruv_nakshatra28_from_tropical_utc(static_cast<const DhruvLskHandle*>(lsk_ptr), lon, ay, &utc, use_nut ? 1 : 0, &out_info);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "nakshatra28", WriteNakshatra28Info(env, out_info));
    return out;
}

napi_value GrahaTropicalLongitudes(napi_env env, napi_callback_info info) {
    size_t argc = 2;
    napi_value args[2];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 2) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double jd = 0.0;
    if (!GetDouble(env, args[1], &jd)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvGrahaLongitudes out_lons{};
    int32_t status = dhruv_graha_tropical_longitudes(static_cast<const DhruvEngineHandle*>(ptr), jd, &out_lons);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        napi_value arr;
        napi_create_array_with_length(env, DHRUV_GRAHA_COUNT, &arr);
        for (uint32_t i = 0; i < DHRUV_GRAHA_COUNT; ++i) {
            napi_set_element(env, arr, i, MakeDouble(env, out_lons.longitudes[i]));
        }
        SetNamed(env, out, "longitudes", arr);
    }
    return out;
}

napi_value NameLookup(napi_env env, napi_callback_info info, const char* (*fn)(uint32_t)) {
    size_t argc = 1;
    napi_value args[1];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 1) return MakeString(env, nullptr);
    uint32_t idx = 0;
    if (!GetUint32(env, args[0], &idx)) return MakeString(env, nullptr);
    return MakeString(env, fn(idx));
}

napi_value RashiName(napi_env env, napi_callback_info info) { return NameLookup(env, info, dhruv_rashi_name); }
napi_value NakshatraName(napi_env env, napi_callback_info info) { return NameLookup(env, info, dhruv_nakshatra_name); }
napi_value Nakshatra28Name(napi_env env, napi_callback_info info) { return NameLookup(env, info, dhruv_nakshatra28_name); }
napi_value MasaName(napi_env env, napi_callback_info info) { return NameLookup(env, info, dhruv_masa_name); }
napi_value AyanaName(napi_env env, napi_callback_info info) { return NameLookup(env, info, dhruv_ayana_name); }
napi_value SamvatsaraName(napi_env env, napi_callback_info info) { return NameLookup(env, info, dhruv_samvatsara_name); }
napi_value TithiName(napi_env env, napi_callback_info info) { return NameLookup(env, info, dhruv_tithi_name); }
napi_value KaranaName(napi_env env, napi_callback_info info) { return NameLookup(env, info, dhruv_karana_name); }
napi_value YogaName(napi_env env, napi_callback_info info) { return NameLookup(env, info, dhruv_yoga_name); }
napi_value VaarName(napi_env env, napi_callback_info info) { return NameLookup(env, info, dhruv_vaar_name); }
napi_value HoraName(napi_env env, napi_callback_info info) { return NameLookup(env, info, dhruv_hora_name); }
napi_value GrahaName(napi_env env, napi_callback_info info) { return NameLookup(env, info, dhruv_graha_name); }
napi_value YoginiName(napi_env env, napi_callback_info info) { return NameLookup(env, info, dhruv_yogini_name); }
napi_value SphutaName(napi_env env, napi_callback_info info) { return NameLookup(env, info, dhruv_sphuta_name); }
napi_value SpecialLagnaName(napi_env env, napi_callback_info info) { return NameLookup(env, info, dhruv_special_lagna_name); }
napi_value ArudhaPadaName(napi_env env, napi_callback_info info) { return NameLookup(env, info, dhruv_arudha_pada_name); }
napi_value UpagrahaName(napi_env env, napi_callback_info info) { return NameLookup(env, info, dhruv_upagraha_name); }

napi_value DegToDms(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value args[1];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 1) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double deg = 0.0;
    if (!GetDouble(env, args[0], &deg)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvDms dms{};
    int32_t status = dhruv_deg_to_dms(deg, &dms);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        napi_value d;
        napi_create_object(env, &d);
        SetNamed(env, d, "degrees", MakeUint32(env, dms.degrees));
        SetNamed(env, d, "minutes", MakeUint32(env, dms.minutes));
        SetNamed(env, d, "seconds", MakeDouble(env, dms.seconds));
        SetNamed(env, out, "dms", d);
    }
    return out;
}

napi_value TithiFromElongation(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value args[1];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 1) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double e = 0.0;
    if (!GetDouble(env, args[0], &e)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvTithiPosition out_t{};
    int32_t status = dhruv_tithi_from_elongation(e, &out_t);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        napi_value t;
        napi_create_object(env, &t);
        SetNamed(env, t, "tithiIndex", MakeInt32(env, out_t.tithi_index));
        SetNamed(env, t, "paksha", MakeInt32(env, out_t.paksha));
        SetNamed(env, t, "tithiInPaksha", MakeInt32(env, out_t.tithi_in_paksha));
        SetNamed(env, t, "degreesInTithi", MakeDouble(env, out_t.degrees_in_tithi));
        SetNamed(env, out, "tithiPosition", t);
    }
    return out;
}

napi_value KaranaFromElongation(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value args[1];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 1) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double e = 0.0;
    if (!GetDouble(env, args[0], &e)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvKaranaPosition out_k{};
    int32_t status = dhruv_karana_from_elongation(e, &out_k);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        napi_value k;
        napi_create_object(env, &k);
        SetNamed(env, k, "karanaIndex", MakeInt32(env, out_k.karana_index));
        SetNamed(env, k, "degreesInKarana", MakeDouble(env, out_k.degrees_in_karana));
        SetNamed(env, out, "karanaPosition", k);
    }
    return out;
}

napi_value YogaFromSum(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value args[1];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 1) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double s = 0.0;
    if (!GetDouble(env, args[0], &s)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvYogaPosition out_y{};
    int32_t status = dhruv_yoga_from_sum(s, &out_y);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        napi_value y;
        napi_create_object(env, &y);
        SetNamed(env, y, "yogaIndex", MakeInt32(env, out_y.yoga_index));
        SetNamed(env, y, "degreesInYoga", MakeDouble(env, out_y.degrees_in_yoga));
        SetNamed(env, out, "yogaPosition", y);
    }
    return out;
}

napi_value VaarFromJd(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value args[1];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 1) return MakeInt32(env, -1);
    double jd = 0.0;
    if (!GetDouble(env, args[0], &jd)) return MakeInt32(env, -1);
    return MakeInt32(env, dhruv_vaar_from_jd(jd));
}

napi_value MasaFromRashiIndex(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value args[1];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 1) return MakeInt32(env, -1);
    uint32_t rashi = 0;
    if (!GetUint32(env, args[0], &rashi)) return MakeInt32(env, -1);
    return MakeInt32(env, dhruv_masa_from_rashi_index(rashi));
}

napi_value AyanaFromSiderealLongitude(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value args[1];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 1) return MakeInt32(env, -1);
    double lon = 0.0;
    if (!GetDouble(env, args[0], &lon)) return MakeInt32(env, -1);
    return MakeInt32(env, dhruv_ayana_from_sidereal_longitude(lon));
}

napi_value SamvatsaraFromYear(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value args[1];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 1) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    int32_t year = 0;
    if (!GetInt32(env, args[0], &year)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvSamvatsaraResult out_s{};
    int32_t status = dhruv_samvatsara_from_year(year, &out_s);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        napi_value s;
        napi_create_object(env, &s);
        SetNamed(env, s, "samvatsaraIndex", MakeInt32(env, out_s.samvatsara_index));
        SetNamed(env, s, "cyclePosition", MakeInt32(env, out_s.cycle_position));
        SetNamed(env, out, "samvatsara", s);
    }
    return out;
}

napi_value NthRashiFrom(napi_env env, napi_callback_info info) {
    size_t argc = 2;
    napi_value args[2];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 2) return MakeInt32(env, -1);
    uint32_t rashi = 0;
    uint32_t offset = 0;
    if (!GetUint32(env, args[0], &rashi) || !GetUint32(env, args[1], &offset)) return MakeInt32(env, -1);
    return MakeInt32(env, dhruv_nth_rashi_from(rashi, offset));
}

napi_value RashiLord(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value args[1];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 1) return MakeInt32(env, -1);
    uint32_t rashi = 0;
    if (!GetUint32(env, args[0], &rashi)) return MakeInt32(env, -1);
    return MakeInt32(env, dhruv_rashi_lord(rashi));
}

napi_value HoraAt(napi_env env, napi_callback_info info) {
    size_t argc = 2;
    napi_value args[2];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 2) return MakeInt32(env, -1);
    uint32_t v = 0;
    uint32_t h = 0;
    if (!GetUint32(env, args[0], &v) || !GetUint32(env, args[1], &h)) return MakeInt32(env, -1);
    return MakeInt32(env, dhruv_hora_at(v, h));
}

napi_value RiseSetConfigDefault(napi_env env, napi_callback_info info) {
    (void)info;
    DhruvRiseSetConfig cfg = dhruv_riseset_config_default();
    napi_value out;
    napi_create_object(env, &out);
    SetNamed(env, out, "useRefraction", MakeBool(env, cfg.use_refraction != 0));
    SetNamed(env, out, "sunLimb", MakeInt32(env, cfg.sun_limb));
    SetNamed(env, out, "altitudeCorrection", MakeBool(env, cfg.altitude_correction != 0));
    return out;
}

napi_value BhavaConfigDefault(napi_env env, napi_callback_info info) {
    (void)info;
    DhruvBhavaConfig cfg = dhruv_bhava_config_default();
    napi_value out;
    napi_create_object(env, &out);
    SetNamed(env, out, "system", MakeInt32(env, cfg.system));
    SetNamed(env, out, "startingPoint", MakeInt32(env, cfg.starting_point));
    SetNamed(env, out, "customStartDeg", MakeDouble(env, cfg.custom_start_deg));
    SetNamed(env, out, "referenceMode", MakeInt32(env, cfg.reference_mode));
    SetNamed(env, out, "outputMode", MakeInt32(env, cfg.output_mode));
    SetNamed(env, out, "ayanamshaSystem", MakeInt32(env, cfg.ayanamsha_system));
    SetNamed(env, out, "useNutation", MakeBool(env, cfg.use_nutation != 0));
    SetNamed(env, out, "referencePlane", MakeInt32(env, cfg.reference_plane));
    return out;
}

napi_value SankrantiConfigDefault(napi_env env, napi_callback_info info) {
    (void)info;
    DhruvSankrantiConfig cfg = dhruv_sankranti_config_default();
    napi_value out;
    napi_create_object(env, &out);
    SetNamed(env, out, "ayanamshaSystem", MakeInt32(env, cfg.ayanamsha_system));
    SetNamed(env, out, "useNutation", MakeBool(env, cfg.use_nutation != 0));
    SetNamed(env, out, "referencePlane", MakeInt32(env, cfg.reference_plane));
    SetNamed(env, out, "stepSizeDays", MakeDouble(env, cfg.step_size_days));
    SetNamed(env, out, "maxIterations", MakeUint32(env, cfg.max_iterations));
    SetNamed(env, out, "convergenceDays", MakeDouble(env, cfg.convergence_days));
    return out;
}

napi_value BhavaSystemCount(napi_env env, napi_callback_info info) {
    (void)info;
    return MakeUint32(env, dhruv_bhava_system_count());
}

napi_value ComputeRiseSet(napi_env env, napi_callback_info info) {
    size_t argc = 7;
    napi_value args[7];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 7) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    void* lsk_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr) || !ReadExternalPtr(env, args[6], &lsk_ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvGeoLocation loc{};
    if (!ReadGeoLocation(env, args[2], &loc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvRiseSetConfig cfg{};
    if (!ReadRiseSetConfig(env, args[3], &cfg)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    int32_t event_code = 0;
    double jd_approx = 0.0;
    if (!GetInt32(env, args[4], &event_code) || !GetDouble(env, args[5], &jd_approx)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvRiseSetResult out_result{};
    int32_t status = dhruv_compute_rise_set(
        static_cast<const DhruvEngineHandle*>(e_ptr),
        static_cast<const DhruvLskHandle*>(lsk_ptr),
        static_cast<const DhruvEopHandle*>(ep_ptr),
        &loc,
        event_code,
        jd_approx,
        &cfg,
        &out_result);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "result", WriteRiseSetResult(env, out_result));
    return out;
}

napi_value ComputeAllEvents(napi_env env, napi_callback_info info) {
    size_t argc = 6;
    napi_value args[6];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 6) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    void* lsk_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr) || !ReadExternalPtr(env, args[5], &lsk_ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvGeoLocation loc{};
    if (!ReadGeoLocation(env, args[2], &loc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvRiseSetConfig cfg{};
    if (!ReadRiseSetConfig(env, args[3], &cfg)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double jd_approx = 0.0;
    if (!GetDouble(env, args[4], &jd_approx)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvRiseSetResult out_results[8]{};
    int32_t status = dhruv_compute_all_events(
        static_cast<const DhruvEngineHandle*>(e_ptr),
        static_cast<const DhruvLskHandle*>(lsk_ptr),
        static_cast<const DhruvEopHandle*>(ep_ptr),
        &loc,
        jd_approx,
        &cfg,
        out_results);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        napi_value arr;
        napi_create_array_with_length(env, 8, &arr);
        for (uint32_t i = 0; i < 8; ++i) napi_set_element(env, arr, i, WriteRiseSetResult(env, out_results[i]));
        SetNamed(env, out, "results", arr);
    }
    return out;
}

napi_value ComputeRiseSetUtc(napi_env env, napi_callback_info info) {
    size_t argc = 7;
    napi_value args[7];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 7) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    void* lsk_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr) || !ReadExternalPtr(env, args[2], &lsk_ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvGeoLocation loc{};
    if (!ReadGeoLocation(env, args[3], &loc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    int32_t event_code = 0;
    if (!GetInt32(env, args[4], &event_code)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvUtcTime utc{};
    if (!ReadUtcTime(env, args[5], &utc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvRiseSetConfig cfg{};
    if (!ReadRiseSetConfig(env, args[6], &cfg)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvRiseSetResultUtc out_result{};
    int32_t status = dhruv_compute_rise_set_utc(
        static_cast<const DhruvEngineHandle*>(e_ptr),
        static_cast<const DhruvLskHandle*>(lsk_ptr),
        static_cast<const DhruvEopHandle*>(ep_ptr),
        &loc,
        event_code,
        &utc,
        &cfg,
        &out_result);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "result", WriteRiseSetResultUtc(env, out_result));
    return out;
}

napi_value ComputeAllEventsUtc(napi_env env, napi_callback_info info) {
    size_t argc = 6;
    napi_value args[6];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 6) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    void* lsk_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr) || !ReadExternalPtr(env, args[2], &lsk_ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvGeoLocation loc{};
    if (!ReadGeoLocation(env, args[3], &loc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvUtcTime utc{};
    if (!ReadUtcTime(env, args[4], &utc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvRiseSetConfig cfg{};
    if (!ReadRiseSetConfig(env, args[5], &cfg)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvRiseSetResultUtc out_results[8]{};
    int32_t status = dhruv_compute_all_events_utc(
        static_cast<const DhruvEngineHandle*>(e_ptr),
        static_cast<const DhruvLskHandle*>(lsk_ptr),
        static_cast<const DhruvEopHandle*>(ep_ptr),
        &loc,
        &utc,
        &cfg,
        out_results);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        napi_value arr;
        napi_create_array_with_length(env, 8, &arr);
        for (uint32_t i = 0; i < 8; ++i) napi_set_element(env, arr, i, WriteRiseSetResultUtc(env, out_results[i]));
        SetNamed(env, out, "results", arr);
    }
    return out;
}

napi_value ComputeBhavas(napi_env env, napi_callback_info info) {
    size_t argc = 6;
    napi_value args[6];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 6) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    void* lsk_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr) || !ReadExternalPtr(env, args[3], &lsk_ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvGeoLocation loc{};
    if (!ReadGeoLocation(env, args[2], &loc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double jd = 0.0;
    if (!GetDouble(env, args[4], &jd)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvBhavaConfig cfg{};
    if (!ReadBhavaConfig(env, args[5], &cfg)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvBhavaResult out_b{};
    int32_t status = dhruv_compute_bhavas(
        static_cast<const DhruvEngineHandle*>(e_ptr),
        static_cast<const DhruvLskHandle*>(lsk_ptr),
        static_cast<const DhruvEopHandle*>(ep_ptr),
        &loc,
        jd,
        &cfg,
        &out_b);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "bhava", WriteBhavaResult(env, out_b));
    return out;
}

napi_value ComputeBhavasUtc(napi_env env, napi_callback_info info) {
    size_t argc = 6;
    napi_value args[6];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 6) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    void* lsk_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr) || !ReadExternalPtr(env, args[2], &lsk_ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvGeoLocation loc{};
    if (!ReadGeoLocation(env, args[3], &loc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvUtcTime utc{};
    if (!ReadUtcTime(env, args[4], &utc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvBhavaConfig cfg{};
    if (!ReadBhavaConfig(env, args[5], &cfg)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvBhavaResult out_b{};
    int32_t status = dhruv_compute_bhavas_utc(
        static_cast<const DhruvEngineHandle*>(e_ptr),
        static_cast<const DhruvLskHandle*>(lsk_ptr),
        static_cast<const DhruvEopHandle*>(ep_ptr),
        &loc,
        &utc,
        &cfg,
        &out_b);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "bhava", WriteBhavaResult(env, out_b));
    return out;
}

napi_value ScalarDegWithLocJd(napi_env env, napi_callback_info info, int32_t (*fn)(const DhruvLskHandle*, const DhruvEopHandle*, const DhruvGeoLocation*, double, double*)) {
    size_t argc = 4;
    napi_value args[4];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 4) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* lsk_ptr = nullptr;
    void* ep_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &lsk_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvGeoLocation loc{};
    if (!ReadGeoLocation(env, args[2], &loc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double jd = 0.0;
    if (!GetDouble(env, args[3], &jd)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double out_deg = 0.0;
    int32_t status = fn(static_cast<const DhruvLskHandle*>(lsk_ptr), static_cast<const DhruvEopHandle*>(ep_ptr), &loc, jd, &out_deg);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "degrees", MakeDouble(env, out_deg));
    return out;
}

napi_value ScalarDegWithLocJdOptionalBhavaConfig(
    napi_env env,
    napi_callback_info info,
    int32_t (*fn_plain)(const DhruvLskHandle*, const DhruvEopHandle*, const DhruvGeoLocation*, double, double*),
    int32_t (*fn_cfg)(const DhruvLskHandle*, const DhruvEopHandle*, const DhruvGeoLocation*, double, const DhruvBhavaConfig*, double*)
) {
    size_t argc = 5;
    napi_value args[5];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 4) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* lsk_ptr = nullptr;
    void* ep_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &lsk_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvGeoLocation loc{};
    if (!ReadGeoLocation(env, args[2], &loc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double jd = 0.0;
    if (!GetDouble(env, args[3], &jd)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double out_deg = 0.0;
    int32_t status = STATUS_OK;
    if (argc >= 5) {
        DhruvBhavaConfig cfg{};
        if (!ReadBhavaConfig(env, args[4], &cfg)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        status = fn_cfg(
            static_cast<const DhruvLskHandle*>(lsk_ptr),
            static_cast<const DhruvEopHandle*>(ep_ptr),
            &loc,
            jd,
            &cfg,
            &out_deg
        );
    } else {
        status = fn_plain(
            static_cast<const DhruvLskHandle*>(lsk_ptr),
            static_cast<const DhruvEopHandle*>(ep_ptr),
            &loc,
            jd,
            &out_deg
        );
    }
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "degrees", MakeDouble(env, out_deg));
    return out;
}

napi_value ScalarDegWithLocUtc(napi_env env, napi_callback_info info, int32_t (*fn)(const DhruvLskHandle*, const DhruvEopHandle*, const DhruvGeoLocation*, const DhruvUtcTime*, double*)) {
    size_t argc = 4;
    napi_value args[4];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 4) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* lsk_ptr = nullptr;
    void* ep_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &lsk_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvGeoLocation loc{};
    if (!ReadGeoLocation(env, args[2], &loc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvUtcTime utc{};
    if (!ReadUtcTime(env, args[3], &utc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double out_deg = 0.0;
    int32_t status = fn(static_cast<const DhruvLskHandle*>(lsk_ptr), static_cast<const DhruvEopHandle*>(ep_ptr), &loc, &utc, &out_deg);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "degrees", MakeDouble(env, out_deg));
    return out;
}

napi_value ScalarDegWithLocUtcOptionalBhavaConfig(
    napi_env env,
    napi_callback_info info,
    int32_t (*fn_plain)(const DhruvLskHandle*, const DhruvEopHandle*, const DhruvGeoLocation*, const DhruvUtcTime*, double*),
    int32_t (*fn_cfg)(const DhruvLskHandle*, const DhruvEopHandle*, const DhruvGeoLocation*, const DhruvUtcTime*, const DhruvBhavaConfig*, double*)
) {
    size_t argc = 5;
    napi_value args[5];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 4) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* lsk_ptr = nullptr;
    void* ep_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &lsk_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvGeoLocation loc{};
    if (!ReadGeoLocation(env, args[2], &loc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvUtcTime utc{};
    if (!ReadUtcTime(env, args[3], &utc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double out_deg = 0.0;
    int32_t status = STATUS_OK;
    if (argc >= 5) {
        DhruvBhavaConfig cfg{};
        if (!ReadBhavaConfig(env, args[4], &cfg)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        status = fn_cfg(
            static_cast<const DhruvLskHandle*>(lsk_ptr),
            static_cast<const DhruvEopHandle*>(ep_ptr),
            &loc,
            &utc,
            &cfg,
            &out_deg
        );
    } else {
        status = fn_plain(
            static_cast<const DhruvLskHandle*>(lsk_ptr),
            static_cast<const DhruvEopHandle*>(ep_ptr),
            &loc,
            &utc,
            &out_deg
        );
    }
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "degrees", MakeDouble(env, out_deg));
    return out;
}

napi_value LagnaDeg(napi_env env, napi_callback_info info) {
    return ScalarDegWithLocJdOptionalBhavaConfig(env, info, dhruv_lagna_deg, dhruv_lagna_deg_with_config);
}
napi_value MCDeg(napi_env env, napi_callback_info info) {
    return ScalarDegWithLocJdOptionalBhavaConfig(env, info, dhruv_mc_deg, dhruv_mc_deg_with_config);
}
napi_value RAMCDeg(napi_env env, napi_callback_info info) { return ScalarDegWithLocJd(env, info, dhruv_ramc_deg); }
napi_value LagnaDegUtc(napi_env env, napi_callback_info info) {
    return ScalarDegWithLocUtcOptionalBhavaConfig(env, info, dhruv_lagna_deg_utc, dhruv_lagna_deg_utc_with_config);
}
napi_value MCDegUtc(napi_env env, napi_callback_info info) {
    return ScalarDegWithLocUtcOptionalBhavaConfig(env, info, dhruv_mc_deg_utc, dhruv_mc_deg_utc_with_config);
}
napi_value RAMCDegUtc(napi_env env, napi_callback_info info) { return ScalarDegWithLocUtc(env, info, dhruv_ramc_deg_utc); }

napi_value RiseSetResultToUtc(napi_env env, napi_callback_info info) {
    size_t argc = 2;
    napi_value args[2];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 2) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* lsk_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &lsk_ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvRiseSetResult r{};
    napi_value v;
    if (!GetNamedProperty(env, args[1], "resultType", &v) || !GetInt32(env, v, &r.result_type)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "eventCode", &v) || !GetInt32(env, v, &r.event_code)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "jdTdb", &v) || !GetDouble(env, v, &r.jd_tdb)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvUtcTime utc{};
    int32_t status = dhruv_riseset_result_to_utc(static_cast<const DhruvLskHandle*>(lsk_ptr), &r, &utc);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "utc", WriteUtcTime(env, utc));
    return out;
}

napi_value ConjunctionConfigDefault(napi_env env, napi_callback_info info) {
    (void)info;
    DhruvConjunctionConfig cfg = dhruv_conjunction_config_default();
    napi_value out;
    napi_create_object(env, &out);
    SetNamed(env, out, "targetSeparationDeg", MakeDouble(env, cfg.target_separation_deg));
    SetNamed(env, out, "stepSizeDays", MakeDouble(env, cfg.step_size_days));
    SetNamed(env, out, "maxIterations", MakeUint32(env, cfg.max_iterations));
    SetNamed(env, out, "convergenceDays", MakeDouble(env, cfg.convergence_days));
    return out;
}

napi_value GrahanConfigDefault(napi_env env, napi_callback_info info) {
    (void)info;
    DhruvGrahanConfig cfg = dhruv_grahan_config_default();
    napi_value out;
    napi_create_object(env, &out);
    SetNamed(env, out, "includePenumbral", MakeBool(env, cfg.include_penumbral != 0));
    SetNamed(env, out, "includePeakDetails", MakeBool(env, cfg.include_peak_details != 0));
    return out;
}

napi_value StationaryConfigDefault(napi_env env, napi_callback_info info) {
    (void)info;
    DhruvStationaryConfig cfg = dhruv_stationary_config_default();
    napi_value out;
    napi_create_object(env, &out);
    SetNamed(env, out, "stepSizeDays", MakeDouble(env, cfg.step_size_days));
    SetNamed(env, out, "maxIterations", MakeUint32(env, cfg.max_iterations));
    SetNamed(env, out, "convergenceDays", MakeDouble(env, cfg.convergence_days));
    SetNamed(env, out, "numericalStepDays", MakeDouble(env, cfg.numerical_step_days));
    return out;
}

napi_value ConjunctionSearch(napi_env env, napi_callback_info info) {
    size_t argc = 3;
    napi_value args[3];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 3) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    void* ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &ptr)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvConjunctionSearchRequest req{};
    DhruvConjunctionConfig cfg = dhruv_conjunction_config_default();
    napi_value v;
    if (!GetNamedProperty(env, args[1], "body1Code", &v) || !GetInt32(env, v, &req.body1_code)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "body2Code", &v) || !GetInt32(env, v, &req.body2_code)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "queryMode", &v) || !GetInt32(env, v, &req.query_mode)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "atJdTdb", &v) || !GetDouble(env, v, &req.at_jd_tdb)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "startJdTdb", &v) || !GetDouble(env, v, &req.start_jd_tdb)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "endJdTdb", &v) || !GetDouble(env, v, &req.end_jd_tdb)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    bool has_cfg = false;
    napi_value cfg_obj;
    if (!GetOptionalNamedProperty(env, args[1], "config", &cfg_obj, &has_cfg)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (has_cfg) {
        bool present = false;
        if (!GetOptionalNamedProperty(env, cfg_obj, "targetSeparationDeg", &v, &present)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        if (present && !GetDouble(env, v, &cfg.target_separation_deg)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        if (!GetOptionalNamedProperty(env, cfg_obj, "stepSizeDays", &v, &present)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        if (present && !GetDouble(env, v, &cfg.step_size_days)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        if (!GetOptionalNamedProperty(env, cfg_obj, "maxIterations", &v, &present)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        if (present && !GetUint32(env, v, &cfg.max_iterations)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        if (!GetOptionalNamedProperty(env, cfg_obj, "convergenceDays", &v, &present)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        if (present && !GetDouble(env, v, &cfg.convergence_days)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }
    req.config = cfg;

    uint32_t capacity = 0;
    if (!GetUint32(env, args[2], &capacity)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvConjunctionEvent out_event{};
    uint8_t found = 0;
    uint32_t out_count = 0;
    std::vector<DhruvConjunctionEvent> events(capacity > 0 ? capacity : 1);
    int32_t status = dhruv_conjunction_search_ex(
        static_cast<const DhruvEngineHandle*>(ptr),
        &req,
        &out_event,
        &found,
        capacity > 0 ? events.data() : nullptr,
        capacity,
        &out_count);

    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "found", MakeBool(env, found != 0));
        SetNamed(env, out, "count", MakeUint32(env, out_count));
        if (found != 0) SetNamed(env, out, "event", WriteConjunctionEvent(env, out_event));
        napi_value arr;
        napi_create_array_with_length(env, out_count, &arr);
        for (uint32_t i = 0; i < out_count; ++i) {
            napi_set_element(env, arr, i, WriteConjunctionEvent(env, events[i]));
        }
        SetNamed(env, out, "events", arr);
    }
    return out;
}

napi_value GrahanSearch(napi_env env, napi_callback_info info) {
    size_t argc = 3;
    napi_value args[3];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 3) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    void* ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvGrahanSearchRequest req{};
    DhruvGrahanConfig cfg = dhruv_grahan_config_default();
    napi_value v;
    if (!GetNamedProperty(env, args[1], "grahanKind", &v) || !GetInt32(env, v, &req.grahan_kind)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "queryMode", &v) || !GetInt32(env, v, &req.query_mode)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "atJdTdb", &v) || !GetDouble(env, v, &req.at_jd_tdb)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "startJdTdb", &v) || !GetDouble(env, v, &req.start_jd_tdb)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "endJdTdb", &v) || !GetDouble(env, v, &req.end_jd_tdb)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    bool has_cfg = false;
    napi_value cfg_obj;
    if (!GetOptionalNamedProperty(env, args[1], "config", &cfg_obj, &has_cfg)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (has_cfg) {
        bool present = false;
        if (!GetOptionalNamedProperty(env, cfg_obj, "includePenumbral", &v, &present)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        if (present) {
            bool b = false;
            if (!GetBool(env, v, &b)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
            cfg.include_penumbral = b ? 1 : 0;
        }
        if (!GetOptionalNamedProperty(env, cfg_obj, "includePeakDetails", &v, &present)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        if (present) {
            bool b = false;
            if (!GetBool(env, v, &b)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
            cfg.include_peak_details = b ? 1 : 0;
        }
    }
    req.config = cfg;

    uint32_t capacity = 0;
    if (!GetUint32(env, args[2], &capacity)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvChandraGrahanResult out_chandra{};
    DhruvSuryaGrahanResult out_surya{};
    uint8_t found = 0;
    uint32_t out_count = 0;
    std::vector<DhruvChandraGrahanResult> ch_events(capacity > 0 ? capacity : 1);
    std::vector<DhruvSuryaGrahanResult> su_events(capacity > 0 ? capacity : 1);

    int32_t status = dhruv_grahan_search_ex(
        static_cast<const DhruvEngineHandle*>(ptr),
        &req,
        &out_chandra,
        &out_surya,
        &found,
        capacity > 0 ? ch_events.data() : nullptr,
        capacity > 0 ? su_events.data() : nullptr,
        capacity,
        &out_count);

    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "found", MakeBool(env, found != 0));
        SetNamed(env, out, "count", MakeUint32(env, out_count));
        SetNamed(env, out, "chandra", WriteChandraGrahanResult(env, out_chandra));
        SetNamed(env, out, "surya", WriteSuryaGrahanResult(env, out_surya));

        napi_value ch_arr;
        napi_create_array_with_length(env, out_count, &ch_arr);
        napi_value su_arr;
        napi_create_array_with_length(env, out_count, &su_arr);
        for (uint32_t i = 0; i < out_count; ++i) {
            napi_set_element(env, ch_arr, i, WriteChandraGrahanResult(env, ch_events[i]));
            napi_set_element(env, su_arr, i, WriteSuryaGrahanResult(env, su_events[i]));
        }
        SetNamed(env, out, "chandraEvents", ch_arr);
        SetNamed(env, out, "suryaEvents", su_arr);
    }
    return out;
}

napi_value MotionSearch(napi_env env, napi_callback_info info) {
    size_t argc = 3;
    napi_value args[3];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 3) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    void* ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvMotionSearchRequest req{};
    DhruvStationaryConfig cfg = dhruv_stationary_config_default();
    napi_value v;
    if (!GetNamedProperty(env, args[1], "bodyCode", &v) || !GetInt32(env, v, &req.body_code)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "motionKind", &v) || !GetInt32(env, v, &req.motion_kind)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "queryMode", &v) || !GetInt32(env, v, &req.query_mode)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "atJdTdb", &v) || !GetDouble(env, v, &req.at_jd_tdb)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "startJdTdb", &v) || !GetDouble(env, v, &req.start_jd_tdb)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "endJdTdb", &v) || !GetDouble(env, v, &req.end_jd_tdb)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    bool has_cfg = false;
    napi_value cfg_obj;
    if (!GetOptionalNamedProperty(env, args[1], "config", &cfg_obj, &has_cfg)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (has_cfg) {
        bool present = false;
        if (!GetOptionalNamedProperty(env, cfg_obj, "stepSizeDays", &v, &present)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        if (present && !GetDouble(env, v, &cfg.step_size_days)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        if (!GetOptionalNamedProperty(env, cfg_obj, "maxIterations", &v, &present)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        if (present && !GetUint32(env, v, &cfg.max_iterations)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        if (!GetOptionalNamedProperty(env, cfg_obj, "convergenceDays", &v, &present)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        if (present && !GetDouble(env, v, &cfg.convergence_days)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        if (!GetOptionalNamedProperty(env, cfg_obj, "numericalStepDays", &v, &present)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        if (present && !GetDouble(env, v, &cfg.numerical_step_days)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }
    req.config = cfg;

    uint32_t capacity = 0;
    if (!GetUint32(env, args[2], &capacity)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvStationaryEvent out_stationary{};
    DhruvMaxSpeedEvent out_max_speed{};
    uint8_t found = 0;
    uint32_t out_count = 0;
    std::vector<DhruvStationaryEvent> st_events(capacity > 0 ? capacity : 1);
    std::vector<DhruvMaxSpeedEvent> mx_events(capacity > 0 ? capacity : 1);

    int32_t status = dhruv_motion_search_ex(
        static_cast<const DhruvEngineHandle*>(ptr),
        &req,
        &out_stationary,
        &out_max_speed,
        &found,
        capacity > 0 ? st_events.data() : nullptr,
        capacity > 0 ? mx_events.data() : nullptr,
        capacity,
        &out_count);

    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "found", MakeBool(env, found != 0));
        SetNamed(env, out, "count", MakeUint32(env, out_count));
        SetNamed(env, out, "stationary", WriteStationaryEvent(env, out_stationary));
        SetNamed(env, out, "maxSpeed", WriteMaxSpeedEvent(env, out_max_speed));

        napi_value st_arr;
        napi_create_array_with_length(env, out_count, &st_arr);
        napi_value mx_arr;
        napi_create_array_with_length(env, out_count, &mx_arr);
        for (uint32_t i = 0; i < out_count; ++i) {
            napi_set_element(env, st_arr, i, WriteStationaryEvent(env, st_events[i]));
            napi_set_element(env, mx_arr, i, WriteMaxSpeedEvent(env, mx_events[i]));
        }
        SetNamed(env, out, "stationaryEvents", st_arr);
        SetNamed(env, out, "maxSpeedEvents", mx_arr);
    }
    return out;
}

napi_value SankrantiSearch(napi_env env, napi_callback_info info) {
    size_t argc = 3;
    napi_value args[3];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 3) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    void* ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvSankrantiSearchRequest req{};
    DhruvSankrantiConfig cfg = dhruv_sankranti_config_default();
    napi_value v;
    if (!GetNamedProperty(env, args[1], "targetKind", &v) || !GetInt32(env, v, &req.target_kind)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "queryMode", &v) || !GetInt32(env, v, &req.query_mode)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "rashiIndex", &v) || !GetInt32(env, v, &req.rashi_index)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "atJdTdb", &v) || !GetDouble(env, v, &req.at_jd_tdb)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "startJdTdb", &v) || !GetDouble(env, v, &req.start_jd_tdb)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "endJdTdb", &v) || !GetDouble(env, v, &req.end_jd_tdb)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    bool has_cfg = false;
    napi_value cfg_obj;
    if (!GetOptionalNamedProperty(env, args[1], "config", &cfg_obj, &has_cfg)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (has_cfg) {
        bool present = false;
        if (!GetOptionalNamedProperty(env, cfg_obj, "ayanamshaSystem", &v, &present)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        if (present && !GetInt32(env, v, &cfg.ayanamsha_system)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        if (!GetOptionalNamedProperty(env, cfg_obj, "useNutation", &v, &present)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        if (present) {
            bool b = false;
            if (!GetBool(env, v, &b)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
            cfg.use_nutation = b ? 1 : 0;
        }
        if (!GetOptionalNamedProperty(env, cfg_obj, "referencePlane", &v, &present)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        if (present && !GetInt32(env, v, &cfg.reference_plane)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        if (!GetOptionalNamedProperty(env, cfg_obj, "stepSizeDays", &v, &present)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        if (present && !GetDouble(env, v, &cfg.step_size_days)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        if (!GetOptionalNamedProperty(env, cfg_obj, "maxIterations", &v, &present)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        if (present && !GetUint32(env, v, &cfg.max_iterations)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        if (!GetOptionalNamedProperty(env, cfg_obj, "convergenceDays", &v, &present)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        if (present && !GetDouble(env, v, &cfg.convergence_days)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }
    req.config = cfg;

    uint32_t capacity = 0;
    if (!GetUint32(env, args[2], &capacity)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvSankrantiEvent out_event{};
    uint8_t found = 0;
    uint32_t out_count = 0;
    std::vector<DhruvSankrantiEvent> events(capacity > 0 ? capacity : 1);
    int32_t status = dhruv_sankranti_search_ex(
        static_cast<const DhruvEngineHandle*>(ptr),
        &req,
        &out_event,
        &found,
        capacity > 0 ? events.data() : nullptr,
        capacity,
        &out_count);

    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "found", MakeBool(env, found != 0));
        SetNamed(env, out, "count", MakeUint32(env, out_count));
        if (found != 0) SetNamed(env, out, "event", WriteSankrantiEvent(env, out_event));
        napi_value arr;
        napi_create_array_with_length(env, out_count, &arr);
        for (uint32_t i = 0; i < out_count; ++i) {
            napi_set_element(env, arr, i, WriteSankrantiEvent(env, events[i]));
        }
        SetNamed(env, out, "events", arr);
    }
    return out;
}

napi_value LunarPhaseSearch(napi_env env, napi_callback_info info) {
    size_t argc = 3;
    napi_value args[3];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 3) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    void* ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &ptr)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvLunarPhaseSearchRequest req{};
    napi_value v;
    if (!GetNamedProperty(env, args[1], "phaseKind", &v) || !GetInt32(env, v, &req.phase_kind)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "queryMode", &v) || !GetInt32(env, v, &req.query_mode)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "atJdTdb", &v) || !GetDouble(env, v, &req.at_jd_tdb)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "startJdTdb", &v) || !GetDouble(env, v, &req.start_jd_tdb)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "endJdTdb", &v) || !GetDouble(env, v, &req.end_jd_tdb)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    uint32_t capacity = 0;
    if (!GetUint32(env, args[2], &capacity)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvLunarPhaseEvent out_event{};
    uint8_t found = 0;
    uint32_t out_count = 0;
    std::vector<DhruvLunarPhaseEvent> events(capacity > 0 ? capacity : 1);

    int32_t status = dhruv_lunar_phase_search_ex(
        static_cast<const DhruvEngineHandle*>(ptr),
        &req,
        &out_event,
        &found,
        capacity > 0 ? events.data() : nullptr,
        capacity,
        &out_count);

    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "found", MakeBool(env, found != 0));
        SetNamed(env, out, "count", MakeUint32(env, out_count));
        if (found != 0) {
            SetNamed(env, out, "event", WriteLunarPhaseEvent(env, out_event));
        }

        napi_value arr;
        napi_create_array_with_length(env, out_count, &arr);
        for (uint32_t i = 0; i < out_count; ++i) {
            napi_set_element(env, arr, i, WriteLunarPhaseEvent(env, events[i]));
        }
        SetNamed(env, out, "events", arr);
    }
    return out;
}

napi_value TithiForDate(napi_env env, napi_callback_info info) {
    size_t argc = 2;
    napi_value args[2];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 2) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    void* ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &ptr)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvUtcTime utc{};
    if (!ReadUtcTime(env, args[1], &utc)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvTithiInfo tithi{};
    int32_t status = dhruv_tithi_for_date(static_cast<const DhruvEngineHandle*>(ptr), &utc, &tithi);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "tithi", WriteTithiInfo(env, tithi));
    }
    return out;
}

napi_value KaranaForDate(napi_env env, napi_callback_info info) {
    size_t argc = 2;
    napi_value args[2];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 2) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    void* ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvUtcTime utc{};
    if (!ReadUtcTime(env, args[1], &utc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvKaranaInfo out_karana{};
    int32_t status = dhruv_karana_for_date(static_cast<const DhruvEngineHandle*>(ptr), &utc, &out_karana);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "karana", WriteKaranaInfo(env, out_karana));
    }
    return out;
}

napi_value YogaForDate(napi_env env, napi_callback_info info) {
    size_t argc = 3;
    napi_value args[3];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 2) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    void* ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvUtcTime utc{};
    if (!ReadUtcTime(env, args[1], &utc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvSankrantiConfig cfg = dhruv_sankranti_config_default();
    if (argc >= 3 && !ReadSankrantiConfig(env, args[2], &cfg)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvYogaInfo out_yoga{};
    int32_t status = dhruv_yoga_for_date(static_cast<const DhruvEngineHandle*>(ptr), &utc, &cfg, &out_yoga);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "yoga", WriteYogaInfo(env, out_yoga));
    }
    return out;
}

napi_value NakshatraForDate(napi_env env, napi_callback_info info) {
    size_t argc = 3;
    napi_value args[3];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 2) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    void* ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvUtcTime utc{};
    if (!ReadUtcTime(env, args[1], &utc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvSankrantiConfig cfg = dhruv_sankranti_config_default();
    if (argc >= 3 && !ReadSankrantiConfig(env, args[2], &cfg)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvPanchangNakshatraInfo out_nak{};
    int32_t status = dhruv_nakshatra_for_date(static_cast<const DhruvEngineHandle*>(ptr), &utc, &cfg, &out_nak);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "nakshatra", WritePanchangNakshatraInfo(env, out_nak));
    }
    return out;
}

napi_value VaarForDate(napi_env env, napi_callback_info info) {
    size_t argc = 5;
    napi_value args[5];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 4) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvUtcTime utc{};
    if (!ReadUtcTime(env, args[2], &utc)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvGeoLocation loc{};
    if (!ReadGeoLocation(env, args[3], &loc)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvRiseSetConfig rise_cfg = dhruv_riseset_config_default();
    if (argc >= 5 && !ReadRiseSetConfig(env, args[4], &rise_cfg)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvVaarInfo info_out{};
    int32_t status = dhruv_vaar_for_date(
        static_cast<const DhruvEngineHandle*>(e_ptr),
        static_cast<const DhruvEopHandle*>(ep_ptr),
        &utc,
        &loc,
        &rise_cfg,
        &info_out);

    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "vaar", WriteVaarInfo(env, info_out));
    }
    return out;
}

napi_value HoraForDate(napi_env env, napi_callback_info info) {
    size_t argc = 5;
    napi_value args[5];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 4) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvUtcTime utc{};
    DhruvGeoLocation loc{};
    if (!ReadUtcTime(env, args[2], &utc) || !ReadGeoLocation(env, args[3], &loc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvRiseSetConfig rise_cfg = dhruv_riseset_config_default();
    if (argc >= 5 && !ReadRiseSetConfig(env, args[4], &rise_cfg)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvHoraInfo out_hora{};
    int32_t status = dhruv_hora_for_date(
        static_cast<const DhruvEngineHandle*>(e_ptr),
        static_cast<const DhruvEopHandle*>(ep_ptr),
        &utc,
        &loc,
        &rise_cfg,
        &out_hora);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "hora", WriteHoraInfo(env, out_hora));
    }
    return out;
}

napi_value GhatikaForDate(napi_env env, napi_callback_info info) {
    size_t argc = 5;
    napi_value args[5];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 4) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvUtcTime utc{};
    DhruvGeoLocation loc{};
    if (!ReadUtcTime(env, args[2], &utc) || !ReadGeoLocation(env, args[3], &loc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvRiseSetConfig rise_cfg = dhruv_riseset_config_default();
    if (argc >= 5 && !ReadRiseSetConfig(env, args[4], &rise_cfg)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvGhatikaInfo out_ghatika{};
    int32_t status = dhruv_ghatika_for_date(
        static_cast<const DhruvEngineHandle*>(e_ptr),
        static_cast<const DhruvEopHandle*>(ep_ptr),
        &utc,
        &loc,
        &rise_cfg,
        &out_ghatika);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "ghatika", WriteGhatikaInfo(env, out_ghatika));
    }
    return out;
}

napi_value MasaForDate(napi_env env, napi_callback_info info) {
    size_t argc = 3;
    napi_value args[3];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 2) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    void* ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvUtcTime utc{};
    if (!ReadUtcTime(env, args[1], &utc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvSankrantiConfig cfg = dhruv_sankranti_config_default();
    if (argc >= 3 && !ReadSankrantiConfig(env, args[2], &cfg)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvMasaInfo out_masa{};
    int32_t status = dhruv_masa_for_date(static_cast<const DhruvEngineHandle*>(ptr), &utc, &cfg, &out_masa);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "masa", WriteMasaInfo(env, out_masa));
    }
    return out;
}

napi_value AyanaForDate(napi_env env, napi_callback_info info) {
    size_t argc = 3;
    napi_value args[3];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 2) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    void* ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvUtcTime utc{};
    if (!ReadUtcTime(env, args[1], &utc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvSankrantiConfig cfg = dhruv_sankranti_config_default();
    if (argc >= 3 && !ReadSankrantiConfig(env, args[2], &cfg)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvAyanaInfo out_ayana{};
    int32_t status = dhruv_ayana_for_date(static_cast<const DhruvEngineHandle*>(ptr), &utc, &cfg, &out_ayana);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "ayana", WriteAyanaInfo(env, out_ayana));
    }
    return out;
}

napi_value VarshaForDate(napi_env env, napi_callback_info info) {
    size_t argc = 3;
    napi_value args[3];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 2) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    void* ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvUtcTime utc{};
    if (!ReadUtcTime(env, args[1], &utc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvSankrantiConfig cfg = dhruv_sankranti_config_default();
    if (argc >= 3 && !ReadSankrantiConfig(env, args[2], &cfg)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvVarshaInfo out_varsha{};
    int32_t status = dhruv_varsha_for_date(static_cast<const DhruvEngineHandle*>(ptr), &utc, &cfg, &out_varsha);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "varsha", WriteVarshaInfo(env, out_varsha));
    }
    return out;
}

napi_value PanchangComputeEx(napi_env env, napi_callback_info info) {
    size_t argc = 4;
    napi_value args[4];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 4) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    void* lsk_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr) || !ReadExternalPtr(env, args[2], &lsk_ptr)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvPanchangComputeRequest req{};
    if (!ReadPanchangComputeRequest(env, args[3], &req)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvPanchangOperationResult result{};
    int32_t status = dhruv_panchang_compute_ex(
        static_cast<const DhruvEngineHandle*>(e_ptr),
        static_cast<const DhruvEopHandle*>(ep_ptr),
        static_cast<const DhruvLskHandle*>(lsk_ptr),
        &req,
        &result);

    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "result", WritePanchangOperationResult(env, result));
    }
    return out;
}

napi_value GrahaSiderealLongitudes(napi_env env, napi_callback_info info) {
    size_t argc = 4;
    napi_value args[4];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 4) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    void* ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &ptr)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    double jd = 0.0;
    uint32_t ayanamsha = 0;
    bool use_nutation = false;
    if (!GetDouble(env, args[1], &jd) || !GetUint32(env, args[2], &ayanamsha) || !GetBool(env, args[3], &use_nutation)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvGrahaLongitudes out_lons{};
    int32_t status = dhruv_graha_sidereal_longitudes(
        static_cast<const DhruvEngineHandle*>(ptr),
        jd,
        ayanamsha,
        use_nutation ? 1 : 0,
        &out_lons);

    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        napi_value arr;
        napi_create_array_with_length(env, DHRUV_GRAHA_COUNT, &arr);
        for (uint32_t i = 0; i < DHRUV_GRAHA_COUNT; ++i) {
            napi_set_element(env, arr, i, MakeDouble(env, out_lons.longitudes[i]));
        }
        SetNamed(env, out, "longitudes", arr);
    }
    return out;
}

napi_value SpecialLagnasForDate(napi_env env, napi_callback_info info) {
    size_t argc = 7;
    napi_value args[7];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 7) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvUtcTime utc{};
    DhruvGeoLocation loc{};
    DhruvRiseSetConfig rise_cfg{};
    if (!ReadUtcTime(env, args[2], &utc) || !ReadGeoLocation(env, args[3], &loc) || !ReadRiseSetConfig(env, args[4], &rise_cfg)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    uint32_t ayanamsha = 0;
    bool use_nutation = false;
    if (!GetUint32(env, args[5], &ayanamsha) || !GetBool(env, args[6], &use_nutation)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvSpecialLagnas out_lagnas{};
    int32_t status = dhruv_special_lagnas_for_date(
        static_cast<const DhruvEngineHandle*>(e_ptr),
        static_cast<const DhruvEopHandle*>(ep_ptr),
        &utc,
        &loc,
        &rise_cfg,
        ayanamsha,
        use_nutation ? 1 : 0,
        &out_lagnas);

    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "lagnas", WriteSpecialLagnas(env, out_lagnas));
    }
    return out;
}

napi_value ArudhaPadasForDate(napi_env env, napi_callback_info info) {
    size_t argc = 6;
    napi_value args[6];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 6) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvUtcTime utc{};
    DhruvGeoLocation loc{};
    if (!ReadUtcTime(env, args[2], &utc) || !ReadGeoLocation(env, args[3], &loc)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    uint32_t ayanamsha = 0;
    bool use_nutation = false;
    if (!GetUint32(env, args[4], &ayanamsha) || !GetBool(env, args[5], &use_nutation)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvArudhaResult out_results[12]{};
    int32_t status = dhruv_arudha_padas_for_date(
        static_cast<const DhruvEngineHandle*>(e_ptr),
        static_cast<const DhruvEopHandle*>(ep_ptr),
        &utc,
        &loc,
        ayanamsha,
        use_nutation ? 1 : 0,
        out_results);

    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        napi_value arr;
        napi_create_array_with_length(env, 12, &arr);
        for (uint32_t i = 0; i < 12; ++i) {
            napi_set_element(env, arr, i, WriteArudhaResult(env, out_results[i]));
        }
        SetNamed(env, out, "results", arr);
    }
    return out;
}

napi_value AllUpagrahasForDate(napi_env env, napi_callback_info info) {
    size_t argc = 7;
    napi_value args[7];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 6) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvUtcTime utc{};
    DhruvGeoLocation loc{};
    if (!ReadUtcTime(env, args[2], &utc) || !ReadGeoLocation(env, args[3], &loc)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    uint32_t ayanamsha = 0;
    bool use_nutation = false;
    if (!GetUint32(env, args[4], &ayanamsha) || !GetBool(env, args[5], &use_nutation)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvAllUpagrahas upagrahas{};
    int32_t status = STATUS_OK;
    if (argc >= 7 && args[6] != nullptr) {
        DhruvTimeUpagrahaConfig upagraha_cfg{};
        if (!ReadTimeUpagrahaConfig(env, args[6], &upagraha_cfg)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        status = dhruv_all_upagrahas_for_date_with_config(
            static_cast<const DhruvEngineHandle*>(e_ptr),
            static_cast<const DhruvEopHandle*>(ep_ptr),
            &utc,
            &loc,
            ayanamsha,
            use_nutation ? 1 : 0,
            &upagraha_cfg,
            &upagrahas);
    } else {
        status = dhruv_all_upagrahas_for_date(
            static_cast<const DhruvEngineHandle*>(e_ptr),
            static_cast<const DhruvEopHandle*>(ep_ptr),
            &utc,
            &loc,
            ayanamsha,
            use_nutation ? 1 : 0,
            &upagrahas);
    }

    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "upagrahas", WriteAllUpagrahas(env, upagrahas));
    }
    return out;
}

napi_value ElongationAt(napi_env env, napi_callback_info info) {
    size_t argc = 2;
    napi_value args[2];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 2) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* ptr = nullptr;
    double jd = 0.0;
    if (!ReadExternalPtr(env, args[0], &ptr) || !GetDouble(env, args[1], &jd)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double out_val = 0.0;
    int32_t status = dhruv_elongation_at(static_cast<const DhruvEngineHandle*>(ptr), jd, &out_val);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "value", MakeDouble(env, out_val));
    return out;
}

napi_value SiderealSumAt(napi_env env, napi_callback_info info) {
    size_t argc = 3;
    napi_value args[3];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 3) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* ptr = nullptr;
    double jd = 0.0;
    DhruvSankrantiConfig cfg{};
    if (!ReadExternalPtr(env, args[0], &ptr) || !GetDouble(env, args[1], &jd) || !ReadSankrantiConfig(env, args[2], &cfg)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }
    double out_val = 0.0;
    int32_t status = dhruv_sidereal_sum_at(static_cast<const DhruvEngineHandle*>(ptr), jd, &cfg, &out_val);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "value", MakeDouble(env, out_val));
    return out;
}

napi_value VedicDaySunrises(napi_env env, napi_callback_info info) {
    size_t argc = 5;
    napi_value args[5];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 5) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    DhruvUtcTime utc{};
    DhruvGeoLocation loc{};
    DhruvRiseSetConfig cfg{};
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr) || !ReadUtcTime(env, args[2], &utc) ||
        !ReadGeoLocation(env, args[3], &loc) || !ReadRiseSetConfig(env, args[4], &cfg)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }
    double sunrise = 0.0;
    double next = 0.0;
    int32_t status = dhruv_vedic_day_sunrises(
        static_cast<const DhruvEngineHandle*>(e_ptr),
        static_cast<const DhruvEopHandle*>(ep_ptr),
        &utc,
        &loc,
        &cfg,
        &sunrise,
        &next);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "sunriseJd", MakeDouble(env, sunrise));
        SetNamed(env, out, "nextSunriseJd", MakeDouble(env, next));
    }
    return out;
}

napi_value BodyEclipticLonLat(napi_env env, napi_callback_info info) {
    size_t argc = 3;
    napi_value args[3];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 3) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* ptr = nullptr;
    int32_t body = 0;
    double jd = 0.0;
    if (!ReadExternalPtr(env, args[0], &ptr) || !GetInt32(env, args[1], &body) || !GetDouble(env, args[2], &jd)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }
    double lon = 0.0;
    double lat = 0.0;
    int32_t status = dhruv_body_ecliptic_lon_lat(static_cast<const DhruvEngineHandle*>(ptr), body, jd, &lon, &lat);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "lonDeg", MakeDouble(env, lon));
        SetNamed(env, out, "latDeg", MakeDouble(env, lat));
    }
    return out;
}

napi_value TithiAt(napi_env env, napi_callback_info info) {
    size_t argc = 3;
    napi_value args[3];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 3) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* ptr = nullptr;
    double jd = 0.0;
    double sunrise = 0.0;
    if (!ReadExternalPtr(env, args[0], &ptr) || !GetDouble(env, args[1], &jd) || !GetDouble(env, args[2], &sunrise)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }
    DhruvTithiInfo tithi{};
    int32_t status = dhruv_tithi_at(static_cast<const DhruvEngineHandle*>(ptr), jd, sunrise, &tithi);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "tithi", WriteTithiInfo(env, tithi));
    return out;
}

napi_value KaranaAt(napi_env env, napi_callback_info info) {
    size_t argc = 3;
    napi_value args[3];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 3) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* ptr = nullptr;
    double jd = 0.0;
    double sunrise = 0.0;
    if (!ReadExternalPtr(env, args[0], &ptr) || !GetDouble(env, args[1], &jd) || !GetDouble(env, args[2], &sunrise)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }
    DhruvKaranaInfo karana{};
    int32_t status = dhruv_karana_at(static_cast<const DhruvEngineHandle*>(ptr), jd, sunrise, &karana);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "karana", WriteKaranaInfo(env, karana));
    return out;
}

napi_value YogaAt(napi_env env, napi_callback_info info) {
    size_t argc = 4;
    napi_value args[4];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 4) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* ptr = nullptr;
    double jd = 0.0;
    double sunrise = 0.0;
    DhruvSankrantiConfig cfg{};
    if (!ReadExternalPtr(env, args[0], &ptr) || !GetDouble(env, args[1], &jd) || !GetDouble(env, args[2], &sunrise) || !ReadSankrantiConfig(env, args[3], &cfg)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }
    DhruvYogaInfo yoga{};
    int32_t status = dhruv_yoga_at(static_cast<const DhruvEngineHandle*>(ptr), jd, sunrise, &cfg, &yoga);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "yoga", WriteYogaInfo(env, yoga));
    return out;
}

napi_value VaarFromSunrises(napi_env env, napi_callback_info info) {
    size_t argc = 3;
    napi_value args[3];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 3) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* ptr = nullptr;
    double sunrise = 0.0;
    double next = 0.0;
    if (!ReadExternalPtr(env, args[0], &ptr) || !GetDouble(env, args[1], &sunrise) || !GetDouble(env, args[2], &next)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }
    DhruvVaarInfo vaar{};
    int32_t status = dhruv_vaar_from_sunrises(static_cast<const DhruvLskHandle*>(ptr), sunrise, next, &vaar);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "vaar", WriteVaarInfo(env, vaar));
    return out;
}

napi_value HoraFromSunrises(napi_env env, napi_callback_info info) {
    size_t argc = 4;
    napi_value args[4];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 4) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* ptr = nullptr;
    double query = 0.0;
    double sunrise = 0.0;
    double next = 0.0;
    if (!ReadExternalPtr(env, args[0], &ptr) || !GetDouble(env, args[1], &query) || !GetDouble(env, args[2], &sunrise) || !GetDouble(env, args[3], &next)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }
    DhruvHoraInfo hora{};
    int32_t status = dhruv_hora_from_sunrises(static_cast<const DhruvLskHandle*>(ptr), query, sunrise, next, &hora);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "hora", WriteHoraInfo(env, hora));
    return out;
}

napi_value GhatikaFromSunrises(napi_env env, napi_callback_info info) {
    size_t argc = 4;
    napi_value args[4];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 4) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* ptr = nullptr;
    double query = 0.0;
    double sunrise = 0.0;
    double next = 0.0;
    if (!ReadExternalPtr(env, args[0], &ptr) || !GetDouble(env, args[1], &query) || !GetDouble(env, args[2], &sunrise) || !GetDouble(env, args[3], &next)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }
    DhruvGhatikaInfo ghatika{};
    int32_t status = dhruv_ghatika_from_sunrises(static_cast<const DhruvLskHandle*>(ptr), query, sunrise, next, &ghatika);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "ghatika", WriteGhatikaInfo(env, ghatika));
    return out;
}

napi_value NakshatraAt(napi_env env, napi_callback_info info) {
    size_t argc = 4;
    napi_value args[4];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 4) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* ptr = nullptr;
    double jd = 0.0;
    double moon_sidereal = 0.0;
    DhruvSankrantiConfig cfg{};
    if (!ReadExternalPtr(env, args[0], &ptr) || !GetDouble(env, args[1], &jd) || !GetDouble(env, args[2], &moon_sidereal) || !ReadSankrantiConfig(env, args[3], &cfg)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }
    DhruvPanchangNakshatraInfo nakshatra{};
    int32_t status = dhruv_nakshatra_at(static_cast<const DhruvEngineHandle*>(ptr), jd, moon_sidereal, &cfg, &nakshatra);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "nakshatra", WritePanchangNakshatraInfo(env, nakshatra));
    return out;
}

napi_value GhatikaFromElapsed(napi_env env, napi_callback_info info) {
    size_t argc = 3;
    napi_value args[3];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 3) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double query = 0.0;
    double sunrise = 0.0;
    double next = 0.0;
    if (!GetDouble(env, args[0], &query) || !GetDouble(env, args[1], &sunrise) || !GetDouble(env, args[2], &next)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }
    int32_t out_value = 0;
    int32_t status = dhruv_ghatika_from_elapsed(query, sunrise, next, &out_value);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "value", MakeInt32(env, out_value));
    return out;
}

napi_value GhatikasSinceSunrise(napi_env env, napi_callback_info info) {
    size_t argc = 2;
    napi_value args[2];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 2) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double query = 0.0;
    double sunrise = 0.0;
    if (!GetDouble(env, args[0], &query) || !GetDouble(env, args[1], &sunrise)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }
    double out_value = 0.0;
    int32_t status = dhruv_ghatikas_since_sunrise(query, sunrise, &out_value);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "value", MakeDouble(env, out_value));
    return out;
}

napi_value AllSphutas(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value args[1];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 1) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvSphutalInputs inputs{};
    if (!ReadSphutalInputs(env, args[0], &inputs)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvSphutalResult result{};
    int32_t status = dhruv_all_sphutas(&inputs, &result);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "result", WriteSphutalResult(env, result));
    return out;
}

napi_value ArudhaPada(napi_env env, napi_callback_info info) {
    size_t argc = 3;
    napi_value args[3];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 3) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double bhava_cusp_lon = 0.0;
    double lord_lon = 0.0;
    uint32_t rashi = 0;
    if (!GetDouble(env, args[0], &bhava_cusp_lon) || !GetDouble(env, args[1], &lord_lon) || !GetUint32(env, args[2], &rashi)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }
    uint8_t out_rashi = static_cast<uint8_t>(rashi);
    double longitude_deg = dhruv_arudha_pada(bhava_cusp_lon, lord_lon, &out_rashi);
    napi_value out = MakeStatusResult(env, STATUS_OK);
    napi_value result;
    napi_create_object(env, &result);
    SetNamed(env, result, "longitudeDeg", MakeDouble(env, longitude_deg));
    SetNamed(env, result, "rashiIndex", MakeUint32(env, out_rashi));
    SetNamed(env, out, "result", result);
    return out;
}

napi_value SunBasedUpagrahas(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value args[1];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 1) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double sun_sid_lon = 0.0;
    if (!GetDouble(env, args[0], &sun_sid_lon)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvAllUpagrahas out_result{};
    int32_t status = dhruv_sun_based_upagrahas(sun_sid_lon, &out_result);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "result", WriteAllUpagrahas(env, out_result));
    return out;
}

napi_value TimeUpagrahaJd(napi_env env, napi_callback_info info) {
    size_t argc = 7;
    napi_value args[7];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 6) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    uint32_t weekday = 0;
    bool is_day = false;
    double sunrise = 0.0;
    double sunset = 0.0;
    double next_sunrise = 0.0;
    uint32_t upagraha = 0;
    if (!GetUint32(env, args[0], &upagraha) || !GetUint32(env, args[1], &weekday) || !GetBool(env, args[2], &is_day) ||
        !GetDouble(env, args[3], &sunrise) || !GetDouble(env, args[4], &sunset) || !GetDouble(env, args[5], &next_sunrise)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }
    double out_jd = 0.0;
    int32_t status = STATUS_OK;
    if (argc >= 7 && args[6] != nullptr) {
        DhruvTimeUpagrahaConfig upagraha_cfg{};
        if (!ReadTimeUpagrahaConfig(env, args[6], &upagraha_cfg)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        status = dhruv_time_upagraha_jd_with_config(
            upagraha,
            weekday,
            is_day ? 1 : 0,
            sunrise,
            sunset,
            next_sunrise,
            &upagraha_cfg,
            &out_jd);
    } else {
        status = dhruv_time_upagraha_jd(
            upagraha,
            weekday,
            is_day ? 1 : 0,
            sunrise,
            sunset,
            next_sunrise,
            &out_jd);
    }
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "jdTdb", MakeDouble(env, out_jd));
    }
    return out;
}

napi_value TimeUpagrahaJdUtc(napi_env env, napi_callback_info info) {
    size_t argc = 7;
    napi_value args[7];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 6) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    DhruvUtcTime utc{};
    DhruvGeoLocation loc{};
    DhruvRiseSetConfig cfg{};
    uint32_t upagraha = 0;
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr) || !ReadUtcTime(env, args[2], &utc) ||
        !ReadGeoLocation(env, args[3], &loc) || !ReadRiseSetConfig(env, args[4], &cfg) || !GetUint32(env, args[5], &upagraha)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }
    double out_jd = 0.0;
    int32_t status = STATUS_OK;
    if (argc >= 7 && args[6] != nullptr) {
        DhruvTimeUpagrahaConfig upagraha_cfg{};
        if (!ReadTimeUpagrahaConfig(env, args[6], &upagraha_cfg)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        status = dhruv_time_upagraha_jd_utc_with_config(
            static_cast<const DhruvEngineHandle*>(e_ptr),
            static_cast<const DhruvEopHandle*>(ep_ptr),
            &utc,
            &loc,
            &cfg,
            &upagraha_cfg,
            upagraha,
            &out_jd);
    } else {
        status = dhruv_time_upagraha_jd_utc(
            static_cast<const DhruvEngineHandle*>(e_ptr),
            static_cast<const DhruvEopHandle*>(ep_ptr),
            &utc,
            &loc,
            &cfg,
            upagraha,
            &out_jd);
    }
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "jdTdb", MakeDouble(env, out_jd));
    }
    return out;
}

#define DEFINE_SCALAR2_WRAPPER(Name, Func)                                 \
napi_value Name(napi_env env, napi_callback_info info) {                   \
    size_t argc = 2;                                                       \
    napi_value args[2];                                                    \
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);            \
    if (argc < 2) return MakeDouble(env, 0.0);                             \
    double a = 0.0, b = 0.0;                                               \
    if (!GetDouble(env, args[0], &a) || !GetDouble(env, args[1], &b)) {    \
        return MakeDouble(env, 0.0);                                       \
    }                                                                       \
    return MakeDouble(env, Func(a, b));                                    \
}

#define DEFINE_SCALAR3_WRAPPER(Name, Func)                                 \
napi_value Name(napi_env env, napi_callback_info info) {                   \
    size_t argc = 3;                                                       \
    napi_value args[3];                                                    \
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);            \
    if (argc < 3) return MakeDouble(env, 0.0);                             \
    double a = 0.0, b = 0.0, c = 0.0;                                      \
    if (!GetDouble(env, args[0], &a) || !GetDouble(env, args[1], &b) ||    \
        !GetDouble(env, args[2], &c)) {                                    \
        return MakeDouble(env, 0.0);                                       \
    }                                                                       \
    return MakeDouble(env, Func(a, b, c));                                 \
}

#define DEFINE_SCALAR4_WRAPPER(Name, Func)                                 \
napi_value Name(napi_env env, napi_callback_info info) {                   \
    size_t argc = 4;                                                       \
    napi_value args[4];                                                    \
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);            \
    if (argc < 4) return MakeDouble(env, 0.0);                             \
    double a = 0.0, b = 0.0, c = 0.0, d = 0.0;                             \
    if (!GetDouble(env, args[0], &a) || !GetDouble(env, args[1], &b) ||    \
        !GetDouble(env, args[2], &c) || !GetDouble(env, args[3], &d)) {    \
        return MakeDouble(env, 0.0);                                       \
    }                                                                       \
    return MakeDouble(env, Func(a, b, c, d));                              \
}

#define DEFINE_SCALAR5_WRAPPER(Name, Func)                                 \
napi_value Name(napi_env env, napi_callback_info info) {                   \
    size_t argc = 5;                                                       \
    napi_value args[5];                                                    \
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);            \
    if (argc < 5) return MakeDouble(env, 0.0);                             \
    double a = 0.0, b = 0.0, c = 0.0, d = 0.0, e = 0.0;                    \
    if (!GetDouble(env, args[0], &a) || !GetDouble(env, args[1], &b) ||    \
        !GetDouble(env, args[2], &c) || !GetDouble(env, args[3], &d) ||    \
        !GetDouble(env, args[4], &e)) {                                    \
        return MakeDouble(env, 0.0);                                       \
    }                                                                       \
    return MakeDouble(env, Func(a, b, c, d, e));                           \
}

DEFINE_SCALAR2_WRAPPER(BhriguBindu, dhruv_bhrigu_bindu)
DEFINE_SCALAR2_WRAPPER(PranaSphuta, dhruv_prana_sphuta)
DEFINE_SCALAR2_WRAPPER(DehaSphuta, dhruv_deha_sphuta)
DEFINE_SCALAR2_WRAPPER(MrityuSphuta, dhruv_mrityu_sphuta)
DEFINE_SCALAR3_WRAPPER(TithiSphuta, dhruv_tithi_sphuta)
DEFINE_SCALAR2_WRAPPER(YogaSphuta, dhruv_yoga_sphuta)
DEFINE_SCALAR2_WRAPPER(YogaSphutaNormalized, dhruv_yoga_sphuta_normalized)
DEFINE_SCALAR3_WRAPPER(RahuTithiSphuta, dhruv_rahu_tithi_sphuta)

napi_value KshetraSphuta(napi_env env, napi_callback_info info) {
    size_t argc = 5;
    napi_value args[5];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 5) return MakeDouble(env, 0.0);
    double moon = 0.0, mars = 0.0, jupiter = 0.0, venus = 0.0, lagna = 0.0;
    if (!GetDouble(env, args[0], &moon) || !GetDouble(env, args[1], &mars) ||
        !GetDouble(env, args[2], &jupiter) || !GetDouble(env, args[3], &venus) ||
        !GetDouble(env, args[4], &lagna)) {
        return MakeDouble(env, 0.0);
    }
    // Keep Node API argument order stable while mapping to C ABI order.
    return MakeDouble(env, dhruv_kshetra_sphuta(venus, moon, mars, jupiter, lagna));
}

DEFINE_SCALAR3_WRAPPER(BeejaSphuta, dhruv_beeja_sphuta)
DEFINE_SCALAR3_WRAPPER(Trisphuta, dhruv_trisphuta)
DEFINE_SCALAR2_WRAPPER(Chatussphuta, dhruv_chatussphuta)
DEFINE_SCALAR2_WRAPPER(Panchasphuta, dhruv_panchasphuta)
DEFINE_SCALAR4_WRAPPER(SookshmaTrisphuta, dhruv_sookshma_trisphuta)
DEFINE_SCALAR2_WRAPPER(AvayogaSphuta, dhruv_avayoga_sphuta)
DEFINE_SCALAR3_WRAPPER(Kunda, dhruv_kunda)

DEFINE_SCALAR2_WRAPPER(BhavaLagna, dhruv_bhava_lagna)
DEFINE_SCALAR2_WRAPPER(HoraLagna, dhruv_hora_lagna)
DEFINE_SCALAR2_WRAPPER(GhatiLagna, dhruv_ghati_lagna)
DEFINE_SCALAR2_WRAPPER(VighatiLagna, dhruv_vighati_lagna)
DEFINE_SCALAR2_WRAPPER(VarnadaLagna, dhruv_varnada_lagna)
DEFINE_SCALAR2_WRAPPER(SreeLagna, dhruv_sree_lagna)
DEFINE_SCALAR2_WRAPPER(PranapadaLagna, dhruv_pranapada_lagna)

napi_value InduLagna(napi_env env, napi_callback_info info) {
    size_t argc = 3;
    napi_value args[3];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 3) return MakeDouble(env, 0.0);
    double moon_lon = 0.0;
    uint32_t lagna_lord = 0;
    uint32_t moon_9th_lord = 0;
    if (!GetDouble(env, args[0], &moon_lon) || !GetUint32(env, args[1], &lagna_lord) || !GetUint32(env, args[2], &moon_9th_lord)) {
        return MakeDouble(env, 0.0);
    }
    return MakeDouble(env, dhruv_indu_lagna(moon_lon, lagna_lord, moon_9th_lord));
}

napi_value CalculateAshtakavarga(napi_env env, napi_callback_info info) {
    size_t argc = 2;
    napi_value args[2];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 2) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    uint8_t graha_rashis[DHRUV_SAPTA_GRAHA_COUNT]{};
    uint32_t lagna_rashi = 0;
    if (!ReadUint8ArrayFixed(env, args[0], graha_rashis, DHRUV_SAPTA_GRAHA_COUNT) || !GetUint32(env, args[1], &lagna_rashi)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }
    DhruvAshtakavargaResult result{};
    int32_t status = dhruv_calculate_ashtakavarga(graha_rashis, static_cast<uint8_t>(lagna_rashi), &result);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "result", WriteAshtakavargaResult(env, result));
    return out;
}

napi_value CalculateBav(napi_env env, napi_callback_info info) {
    size_t argc = 3;
    napi_value args[3];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 3) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    uint32_t graha_index = 0;
    uint8_t graha_rashis[DHRUV_SAPTA_GRAHA_COUNT]{};
    uint32_t lagna_rashi = 0;
    if (!GetUint32(env, args[0], &graha_index) || !ReadUint8ArrayFixed(env, args[1], graha_rashis, DHRUV_SAPTA_GRAHA_COUNT) || !GetUint32(env, args[2], &lagna_rashi)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }
    DhruvBhinnaAshtakavarga result{};
    int32_t status = dhruv_calculate_bav(static_cast<uint8_t>(graha_index), graha_rashis, static_cast<uint8_t>(lagna_rashi), &result);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "result", WriteBhinnaAshtakavarga(env, result));
    return out;
}

napi_value CalculateAllBav(napi_env env, napi_callback_info info) {
    size_t argc = 2;
    napi_value args[2];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 2) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    uint8_t graha_rashis[DHRUV_SAPTA_GRAHA_COUNT]{};
    uint32_t lagna_rashi = 0;
    if (!ReadUint8ArrayFixed(env, args[0], graha_rashis, DHRUV_SAPTA_GRAHA_COUNT) || !GetUint32(env, args[1], &lagna_rashi)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }
    DhruvBhinnaAshtakavarga results[DHRUV_SAPTA_GRAHA_COUNT]{};
    int32_t status = dhruv_calculate_all_bav(graha_rashis, static_cast<uint8_t>(lagna_rashi), results);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        napi_value arr;
        napi_create_array_with_length(env, DHRUV_SAPTA_GRAHA_COUNT, &arr);
        for (uint32_t i = 0; i < DHRUV_SAPTA_GRAHA_COUNT; ++i) {
            napi_set_element(env, arr, i, WriteBhinnaAshtakavarga(env, results[i]));
        }
        SetNamed(env, out, "results", arr);
    }
    return out;
}

napi_value CalculateSav(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value args[1];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 1) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    bool is_array = false;
    if (napi_is_array(env, args[0], &is_array) != napi_ok || !is_array) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    uint32_t len = 0;
    if (napi_get_array_length(env, args[0], &len) != napi_ok || len < DHRUV_SAPTA_GRAHA_COUNT) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvBhinnaAshtakavarga bavs[DHRUV_SAPTA_GRAHA_COUNT]{};
    for (uint32_t i = 0; i < DHRUV_SAPTA_GRAHA_COUNT; ++i) {
        napi_value item;
        if (napi_get_element(env, args[0], i, &item) != napi_ok || !ReadBhinnaAshtakavarga(env, item, &bavs[i])) {
            return MakeStatusResult(env, STATUS_INVALID_INPUT);
        }
    }
    DhruvSarvaAshtakavarga result{};
    int32_t status = dhruv_calculate_sav(bavs, &result);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "result", WriteSarvaAshtakavarga(env, result));
    return out;
}

napi_value TrikonaSodhana(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value args[1];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 1) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    uint8_t totals[12]{};
    uint8_t out_totals[12]{};
    if (!ReadUint8ArrayFixed(env, args[0], totals, 12)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    int32_t status = dhruv_trikona_sodhana(totals, out_totals);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        napi_value arr;
        napi_create_array_with_length(env, 12, &arr);
        for (uint32_t i = 0; i < 12; ++i) {
            napi_set_element(env, arr, i, MakeUint32(env, out_totals[i]));
        }
        SetNamed(env, out, "result", arr);
    }
    return out;
}

napi_value EkadhipatyaSodhana(napi_env env, napi_callback_info info) {
    size_t argc = 3;
    napi_value args[3];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 3) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    uint8_t totals[12]{};
    uint8_t graha_rashis[DHRUV_SAPTA_GRAHA_COUNT]{};
    uint8_t out_totals[12]{};
    uint32_t lagna_rashi = 0;
    if (!ReadUint8ArrayFixed(env, args[0], totals, 12) || !ReadUint8ArrayFixed(env, args[1], graha_rashis, DHRUV_SAPTA_GRAHA_COUNT) || !GetUint32(env, args[2], &lagna_rashi)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }
    int32_t status = dhruv_ekadhipatya_sodhana(totals, graha_rashis, static_cast<uint8_t>(lagna_rashi), out_totals);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        napi_value arr;
        napi_create_array_with_length(env, 12, &arr);
        for (uint32_t i = 0; i < 12; ++i) {
            napi_set_element(env, arr, i, MakeUint32(env, out_totals[i]));
        }
        SetNamed(env, out, "result", arr);
    }
    return out;
}

napi_value AshtakavargaForDate(napi_env env, napi_callback_info info) {
    size_t argc = 6;
    napi_value args[6];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 6) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    DhruvUtcTime utc{};
    DhruvGeoLocation loc{};
    uint32_t ayanamsha = 0;
    bool use_nutation = false;
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr) || !ReadUtcTime(env, args[2], &utc) ||
        !ReadGeoLocation(env, args[3], &loc) || !GetUint32(env, args[4], &ayanamsha) || !GetBool(env, args[5], &use_nutation)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }
    DhruvAshtakavargaResult result{};
    int32_t status = dhruv_ashtakavarga_for_date(
        static_cast<const DhruvEngineHandle*>(e_ptr),
        static_cast<const DhruvEopHandle*>(ep_ptr),
        &utc,
        &loc,
        ayanamsha,
        use_nutation ? 1 : 0,
        &result);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "result", WriteAshtakavargaResult(env, result));
    return out;
}

napi_value GrahaDrishti(napi_env env, napi_callback_info info) {
    size_t argc = 3;
    napi_value args[3];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 3) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    uint32_t graha = 0;
    double source = 0.0;
    double target = 0.0;
    if (!GetUint32(env, args[0], &graha) || !GetDouble(env, args[1], &source) || !GetDouble(env, args[2], &target)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }
    DhruvDrishtiEntry result{};
    int32_t status = dhruv_graha_drishti(graha, source, target, &result);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "result", WriteDrishtiEntry(env, result));
    return out;
}

napi_value GrahaDrishtiMatrix(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value args[1];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 1) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double longitudes[DHRUV_GRAHA_COUNT]{};
    if (!ReadDoubleArrayFixed(env, args[0], longitudes, DHRUV_GRAHA_COUNT)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvGrahaDrishtiMatrix result{};
    int32_t status = dhruv_graha_drishti_matrix(longitudes, &result);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "result", WriteGrahaDrishtiMatrix(env, result));
    return out;
}

napi_value DrishtiForDate(napi_env env, napi_callback_info info) {
    size_t argc = 9;
    napi_value args[9];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 9) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    DhruvUtcTime utc{};
    DhruvGeoLocation loc{};
    DhruvBhavaConfig bhava_cfg{};
    DhruvRiseSetConfig rise_cfg{};
    uint32_t ayanamsha = 0;
    bool use_nutation = false;
    DhruvDrishtiConfig cfg{};
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr) || !ReadUtcTime(env, args[2], &utc) ||
        !ReadGeoLocation(env, args[3], &loc) || !ReadBhavaConfig(env, args[4], &bhava_cfg) || !ReadRiseSetConfig(env, args[5], &rise_cfg) ||
        !GetUint32(env, args[6], &ayanamsha) || !GetBool(env, args[7], &use_nutation) || !ReadDrishtiConfig(env, args[8], &cfg)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }
    DhruvDrishtiResult result{};
    int32_t status = dhruv_drishti(
        static_cast<const DhruvEngineHandle*>(e_ptr),
        static_cast<const DhruvEopHandle*>(ep_ptr),
        &utc,
        &loc,
        &bhava_cfg,
        &rise_cfg,
        ayanamsha,
        use_nutation ? 1 : 0,
        &cfg,
        &result);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "result", WriteDrishtiResult(env, result));
    return out;
}

napi_value GrahaPositionsForDate(napi_env env, napi_callback_info info) {
    size_t argc = 8;
    napi_value args[8];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 8) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    DhruvUtcTime utc{};
    DhruvGeoLocation loc{};
    DhruvBhavaConfig bhava_cfg{};
    uint32_t ayanamsha = 0;
    bool use_nutation = false;
    DhruvGrahaPositionsConfig cfg{};
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr) || !ReadUtcTime(env, args[2], &utc) ||
        !ReadGeoLocation(env, args[3], &loc) || !ReadBhavaConfig(env, args[4], &bhava_cfg) || !GetUint32(env, args[5], &ayanamsha) ||
        !GetBool(env, args[6], &use_nutation) || !ReadGrahaPositionsConfig(env, args[7], &cfg)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }
    DhruvGrahaPositions result{};
    int32_t status = dhruv_graha_positions(
        static_cast<const DhruvEngineHandle*>(e_ptr),
        static_cast<const DhruvEopHandle*>(ep_ptr),
        &utc,
        &loc,
        &bhava_cfg,
        ayanamsha,
        use_nutation ? 1 : 0,
        &cfg,
        &result);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "result", WriteGrahaPositions(env, result));
    return out;
}

napi_value CoreBindusForDate(napi_env env, napi_callback_info info) {
    size_t argc = 9;
    napi_value args[9];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 9) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    DhruvUtcTime utc{};
    DhruvGeoLocation loc{};
    DhruvBhavaConfig bhava_cfg{};
    DhruvRiseSetConfig rise_cfg{};
    uint32_t ayanamsha = 0;
    bool use_nutation = false;
    DhruvBindusConfig cfg{};
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr) || !ReadUtcTime(env, args[2], &utc) ||
        !ReadGeoLocation(env, args[3], &loc) || !ReadBhavaConfig(env, args[4], &bhava_cfg) || !ReadRiseSetConfig(env, args[5], &rise_cfg) ||
        !GetUint32(env, args[6], &ayanamsha) || !GetBool(env, args[7], &use_nutation) || !ReadBindusConfig(env, args[8], &cfg)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }
    DhruvBindusResult result{};
    int32_t status = dhruv_core_bindus(
        static_cast<const DhruvEngineHandle*>(e_ptr),
        static_cast<const DhruvEopHandle*>(ep_ptr),
        &utc,
        &loc,
        &bhava_cfg,
        &rise_cfg,
        ayanamsha,
        use_nutation ? 1 : 0,
        &cfg,
        &result);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "result", WriteBindusResult(env, result));
    return out;
}

napi_value AmshaLongitude(napi_env env, napi_callback_info info) {
    size_t argc = 3;
    napi_value args[3];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 3) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double sidereal = 0.0;
    uint32_t amsha_code = 0;
    uint32_t variation = 0;
    if (!GetDouble(env, args[0], &sidereal) || !GetUint32(env, args[1], &amsha_code) || !GetUint32(env, args[2], &variation)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }
    double out_lon = 0.0;
    int32_t status = dhruv_amsha_longitude(sidereal, static_cast<uint16_t>(amsha_code), static_cast<uint8_t>(variation), &out_lon);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "longitudeDeg", MakeDouble(env, out_lon));
    return out;
}

napi_value AmshaRashiInfo(napi_env env, napi_callback_info info) {
    size_t argc = 3;
    napi_value args[3];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 3) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double sidereal = 0.0;
    uint32_t amsha_code = 0;
    uint32_t variation = 0;
    if (!GetDouble(env, args[0], &sidereal) || !GetUint32(env, args[1], &amsha_code) || !GetUint32(env, args[2], &variation)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }
    DhruvRashiInfo result{};
    int32_t status = dhruv_amsha_rashi_info(sidereal, static_cast<uint16_t>(amsha_code), static_cast<uint8_t>(variation), &result);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "rashi", WriteRashiInfo(env, result));
    return out;
}

napi_value AmshaLongitudes(napi_env env, napi_callback_info info) {
    size_t argc = 3;
    napi_value args[3];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 3) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    double sidereal = 0.0;
    if (!GetDouble(env, args[0], &sidereal)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    bool is_codes_array = false;
    bool is_vars_array = false;
    if (napi_is_array(env, args[1], &is_codes_array) != napi_ok || !is_codes_array) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (napi_is_array(env, args[2], &is_vars_array) != napi_ok || !is_vars_array) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    uint32_t codes_len = 0;
    uint32_t vars_len = 0;
    if (napi_get_array_length(env, args[1], &codes_len) != napi_ok || napi_get_array_length(env, args[2], &vars_len) != napi_ok || codes_len != vars_len || codes_len == 0) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    std::vector<uint16_t> codes(codes_len);
    std::vector<uint8_t> vars(codes_len);
    std::vector<double> out_vals(codes_len);
    for (uint32_t i = 0; i < codes_len; ++i) {
        napi_value cv;
        napi_value vv;
        uint32_t c = 0;
        uint32_t v = 0;
        if (napi_get_element(env, args[1], i, &cv) != napi_ok || napi_get_element(env, args[2], i, &vv) != napi_ok || !GetUint32(env, cv, &c) || !GetUint32(env, vv, &v)) {
            return MakeStatusResult(env, STATUS_INVALID_INPUT);
        }
        codes[i] = static_cast<uint16_t>(c);
        vars[i] = static_cast<uint8_t>(v);
    }

    int32_t status = dhruv_amsha_longitudes(sidereal, codes.data(), vars.data(), codes_len, out_vals.data());
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        napi_value arr;
        napi_create_array_with_length(env, codes_len, &arr);
        for (uint32_t i = 0; i < codes_len; ++i) {
            napi_set_element(env, arr, i, MakeDouble(env, out_vals[i]));
        }
        SetNamed(env, out, "longitudes", arr);
    }
    return out;
}

napi_value AmshaChartForDate(napi_env env, napi_callback_info info) {
    size_t argc = 11;
    napi_value args[11];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 11) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    DhruvUtcTime utc{};
    DhruvGeoLocation loc{};
    DhruvBhavaConfig bhava_cfg{};
    DhruvRiseSetConfig rise_cfg{};
    uint32_t ayanamsha = 0;
    bool use_nutation = false;
    uint32_t amsha_code = 0;
    uint32_t variation = 0;
    DhruvAmshaChartScope scope{};
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr) || !ReadUtcTime(env, args[2], &utc) ||
        !ReadGeoLocation(env, args[3], &loc) || !ReadBhavaConfig(env, args[4], &bhava_cfg) || !ReadRiseSetConfig(env, args[5], &rise_cfg) ||
        !GetUint32(env, args[6], &ayanamsha) || !GetBool(env, args[7], &use_nutation) || !GetUint32(env, args[8], &amsha_code) ||
        !GetUint32(env, args[9], &variation) || !ReadAmshaChartScope(env, args[10], &scope)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }
    DhruvAmshaChart result{};
    int32_t status = dhruv_amsha_chart_for_date(
        static_cast<const DhruvEngineHandle*>(e_ptr),
        static_cast<const DhruvEopHandle*>(ep_ptr),
        &utc,
        &loc,
        &bhava_cfg,
        &rise_cfg,
        ayanamsha,
        use_nutation ? 1 : 0,
        static_cast<uint16_t>(amsha_code),
        static_cast<uint8_t>(variation),
        &scope,
        &result);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "result", WriteAmshaChart(env, result));
    return out;
}

napi_value ShadbalaForDate(napi_env env, napi_callback_info info) {
    size_t argc = 8;
    napi_value args[8];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 6) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvUtcTime utc{};
    DhruvGeoLocation loc{};
    if (!ReadUtcTime(env, args[2], &utc) || !ReadGeoLocation(env, args[3], &loc)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    uint32_t ayanamsha = 0;
    bool use_nutation = false;
    if (!GetUint32(env, args[4], &ayanamsha) || !GetBool(env, args[5], &use_nutation)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvBhavaConfig bhava_cfg = dhruv_bhava_config_default();
    DhruvRiseSetConfig rise_cfg = dhruv_riseset_config_default();
    if (argc >= 7 && !ReadBhavaConfig(env, args[6], &bhava_cfg)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (argc >= 8 && !ReadRiseSetConfig(env, args[7], &rise_cfg)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    DhruvShadbalaResult out_result{};

    int32_t status = dhruv_shadbala_for_date(
        static_cast<const DhruvEngineHandle*>(e_ptr),
        static_cast<const DhruvEopHandle*>(ep_ptr),
        &utc,
        &loc,
        &bhava_cfg,
        &rise_cfg,
        ayanamsha,
        use_nutation ? 1 : 0,
        &out_result);

    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "result", WriteShadbalaResult(env, out_result));
    return out;
}

napi_value CalculateBhavaBala(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value args[1];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 1) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvBhavaBalaInputs inputs{};
    if (!ReadBhavaBalaInputs(env, args[0], &inputs)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvBhavaBalaResult out_result{};
    int32_t status = dhruv_calculate_bhavabala(&inputs, &out_result);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "result", WriteBhavaBalaResult(env, out_result));
    return out;
}

napi_value BhavaBalaForDate(napi_env env, napi_callback_info info) {
    size_t argc = 8;
    napi_value args[8];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 6) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvUtcTime utc{};
    DhruvGeoLocation loc{};
    if (!ReadUtcTime(env, args[2], &utc) || !ReadGeoLocation(env, args[3], &loc)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    uint32_t ayanamsha = 0;
    bool use_nutation = false;
    if (!GetUint32(env, args[4], &ayanamsha) || !GetBool(env, args[5], &use_nutation)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvBhavaConfig bhava_cfg = dhruv_bhava_config_default();
    DhruvRiseSetConfig rise_cfg = dhruv_riseset_config_default();
    if (argc >= 7 && !ReadBhavaConfig(env, args[6], &bhava_cfg)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (argc >= 8 && !ReadRiseSetConfig(env, args[7], &rise_cfg)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvBhavaBalaResult out_result{};
    int32_t status = dhruv_bhavabala_for_date(
        static_cast<const DhruvEngineHandle*>(e_ptr),
        static_cast<const DhruvEopHandle*>(ep_ptr),
        &utc,
        &loc,
        &bhava_cfg,
        &rise_cfg,
        ayanamsha,
        use_nutation ? 1 : 0,
        &out_result);

    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "result", WriteBhavaBalaResult(env, out_result));
    return out;
}

napi_value VimsopakaForDate(napi_env env, napi_callback_info info) {
    size_t argc = 7;
    napi_value args[7];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 7) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvUtcTime utc{};
    DhruvGeoLocation loc{};
    if (!ReadUtcTime(env, args[2], &utc) || !ReadGeoLocation(env, args[3], &loc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    uint32_t ayanamsha = 0;
    bool use_nutation = false;
    uint32_t node_policy = 0;
    if (!GetUint32(env, args[4], &ayanamsha) || !GetBool(env, args[5], &use_nutation) || !GetUint32(env, args[6], &node_policy)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvVimsopakaResult out_result{};
    int32_t status = dhruv_vimsopaka_for_date(
        static_cast<const DhruvEngineHandle*>(e_ptr),
        static_cast<const DhruvEopHandle*>(ep_ptr),
        &utc,
        &loc,
        ayanamsha,
        use_nutation ? 1 : 0,
        node_policy,
        &out_result);

    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "result", WriteVimsopakaResult(env, out_result));
    return out;
}

napi_value BalasForDate(napi_env env, napi_callback_info info) {
    size_t argc = 9;
    napi_value args[9];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 9) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvUtcTime utc{};
    DhruvGeoLocation loc{};
    DhruvBhavaConfig bhava_cfg{};
    DhruvRiseSetConfig rise_cfg{};
    if (!ReadUtcTime(env, args[2], &utc) || !ReadGeoLocation(env, args[3], &loc) || !ReadBhavaConfig(env, args[4], &bhava_cfg) || !ReadRiseSetConfig(env, args[5], &rise_cfg)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    uint32_t ayanamsha = 0;
    bool use_nutation = false;
    uint32_t node_policy = 0;
    if (!GetUint32(env, args[6], &ayanamsha) || !GetBool(env, args[7], &use_nutation) || !GetUint32(env, args[8], &node_policy)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvBalaBundleResult out_result{};
    int32_t status = dhruv_balas_for_date(
        static_cast<const DhruvEngineHandle*>(e_ptr),
        static_cast<const DhruvEopHandle*>(ep_ptr),
        &utc,
        &loc,
        &bhava_cfg,
        &rise_cfg,
        ayanamsha,
        use_nutation ? 1 : 0,
        node_policy,
        &out_result);

    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        napi_value result;
        napi_create_object(env, &result);
        SetNamed(env, result, "shadbala", WriteShadbalaResult(env, out_result.shadbala));
        SetNamed(env, result, "vimsopaka", WriteVimsopakaResult(env, out_result.vimsopaka));
        SetNamed(env, result, "ashtakavarga", WriteAshtakavargaResult(env, out_result.ashtakavarga));
        SetNamed(env, result, "bhavabala", WriteBhavaBalaResult(env, out_result.bhavabala));
        SetNamed(env, out, "result", result);
    }
    return out;
}

napi_value AvasthaForDate(napi_env env, napi_callback_info info) {
    size_t argc = 9;
    napi_value args[9];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 9) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvUtcTime utc{};
    DhruvGeoLocation loc{};
    DhruvBhavaConfig bhava_cfg{};
    DhruvRiseSetConfig rise_cfg{};
    if (!ReadUtcTime(env, args[2], &utc) || !ReadGeoLocation(env, args[3], &loc) || !ReadBhavaConfig(env, args[4], &bhava_cfg) || !ReadRiseSetConfig(env, args[5], &rise_cfg)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    uint32_t ayanamsha = 0;
    bool use_nutation = false;
    uint32_t node_policy = 0;
    if (!GetUint32(env, args[6], &ayanamsha) || !GetBool(env, args[7], &use_nutation) || !GetUint32(env, args[8], &node_policy)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvAllGrahaAvasthas out_result{};
    int32_t status = dhruv_avastha_for_date(
        static_cast<const DhruvEngineHandle*>(e_ptr),
        static_cast<const DhruvEopHandle*>(ep_ptr),
        &utc,
        &loc,
        &bhava_cfg,
        &rise_cfg,
        ayanamsha,
        use_nutation ? 1 : 0,
        node_policy,
        &out_result);

    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "result", WriteAllGrahaAvasthas(env, out_result));
    return out;
}

napi_value CharakarakaForDate(napi_env env, napi_callback_info info) {
    size_t argc = 6;
    napi_value args[6];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 6) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvUtcTime utc{};
    if (!ReadUtcTime(env, args[2], &utc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    uint32_t ayanamsha = 0;
    bool use_nutation = false;
    uint32_t scheme = 0;
    if (!GetUint32(env, args[3], &ayanamsha) || !GetBool(env, args[4], &use_nutation) || !GetUint32(env, args[5], &scheme)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvCharakarakaResult out_result{};
    int32_t status = dhruv_charakaraka_for_date(
        static_cast<const DhruvEngineHandle*>(e_ptr),
        static_cast<const DhruvEopHandle*>(ep_ptr),
        &utc,
        ayanamsha,
        use_nutation ? 1 : 0,
        static_cast<uint8_t>(scheme),
        &out_result);

    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) SetNamed(env, out, "result", WriteCharakarakaResult(env, out_result));
    return out;
}

napi_value FullKundaliSummaryForDate(napi_env env, napi_callback_info info) {
    size_t argc = 6;
    napi_value args[6];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 6) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvUtcTime utc{};
    DhruvGeoLocation loc{};
    if (!ReadUtcTime(env, args[2], &utc) || !ReadGeoLocation(env, args[3], &loc)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    uint32_t ayanamsha = 0;
    bool use_nutation = false;
    if (!GetUint32(env, args[4], &ayanamsha) || !GetBool(env, args[5], &use_nutation)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvBhavaConfig bhava_cfg = dhruv_bhava_config_default();
    DhruvRiseSetConfig rise_cfg = dhruv_riseset_config_default();
    DhruvFullKundaliConfig full_cfg = dhruv_full_kundali_config_default();
    DhruvFullKundaliResult out_result{};

    int32_t status = dhruv_full_kundali_for_date(
        static_cast<const DhruvEngineHandle*>(e_ptr),
        static_cast<const DhruvEopHandle*>(ep_ptr),
        &utc,
        &loc,
        &bhava_cfg,
        &rise_cfg,
        ayanamsha,
        use_nutation ? 1 : 0,
        &full_cfg,
        &out_result);

    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "ayanamshaDeg", MakeDouble(env, out_result.ayanamsha_deg));
        SetNamed(env, out, "grahaPositionsValid", MakeBool(env, out_result.graha_positions_valid != 0));
        SetNamed(env, out, "charakarakaValid", MakeBool(env, out_result.charakaraka_valid != 0));
        SetNamed(env, out, "panchangValid", MakeBool(env, out_result.panchang_valid != 0));
        SetNamed(env, out, "dashaSnapshotCount", MakeUint32(env, out_result.dasha_snapshot_count));
        dhruv_full_kundali_result_free(&out_result);
    }

    return out;
}

napi_value FullKundaliConfigDefault(napi_env env, napi_callback_info info) {
    (void)info;
    DhruvFullKundaliConfig cfg = dhruv_full_kundali_config_default();
    napi_value obj;
    napi_create_object(env, &obj);

    SetNamed(env, obj, "includeBhavaCusps", MakeBool(env, cfg.include_bhava_cusps != 0));
    SetNamed(env, obj, "includeGrahaPositions", MakeBool(env, cfg.include_graha_positions != 0));
    SetNamed(env, obj, "includeBindus", MakeBool(env, cfg.include_bindus != 0));
    SetNamed(env, obj, "includeDrishti", MakeBool(env, cfg.include_drishti != 0));
    SetNamed(env, obj, "includeAshtakavarga", MakeBool(env, cfg.include_ashtakavarga != 0));
    SetNamed(env, obj, "includeUpagrahas", MakeBool(env, cfg.include_upagrahas != 0));
    SetNamed(env, obj, "includeSphutas", MakeBool(env, cfg.include_sphutas != 0));
    SetNamed(env, obj, "includeSpecialLagnas", MakeBool(env, cfg.include_special_lagnas != 0));
    SetNamed(env, obj, "includeAmshas", MakeBool(env, cfg.include_amshas != 0));
    SetNamed(env, obj, "includeShadbala", MakeBool(env, cfg.include_shadbala != 0));
    SetNamed(env, obj, "includeBhavaBala", MakeBool(env, cfg.include_bhavabala != 0));
    SetNamed(env, obj, "includeVimsopaka", MakeBool(env, cfg.include_vimsopaka != 0));
    SetNamed(env, obj, "includeAvastha", MakeBool(env, cfg.include_avastha != 0));
    SetNamed(env, obj, "includeCharakaraka", MakeBool(env, cfg.include_charakaraka != 0));
    SetNamed(env, obj, "charakarakaScheme", MakeUint32(env, cfg.charakaraka_scheme));
    SetNamed(env, obj, "nodeDignityPolicy", MakeUint32(env, cfg.node_dignity_policy));
    SetNamed(env, obj, "upagrahaConfig", WriteTimeUpagrahaConfig(env, cfg.upagraha_config));

    napi_value graha_cfg;
    napi_create_object(env, &graha_cfg);
    SetNamed(env, graha_cfg, "includeNakshatra", MakeBool(env, cfg.graha_positions_config.include_nakshatra != 0));
    SetNamed(env, graha_cfg, "includeLagna", MakeBool(env, cfg.graha_positions_config.include_lagna != 0));
    SetNamed(env, graha_cfg, "includeOuterPlanets", MakeBool(env, cfg.graha_positions_config.include_outer_planets != 0));
    SetNamed(env, graha_cfg, "includeBhava", MakeBool(env, cfg.graha_positions_config.include_bhava != 0));
    SetNamed(env, obj, "grahaPositionsConfig", graha_cfg);

    napi_value bindus_cfg;
    napi_create_object(env, &bindus_cfg);
    SetNamed(env, bindus_cfg, "includeNakshatra", MakeBool(env, cfg.bindus_config.include_nakshatra != 0));
    SetNamed(env, bindus_cfg, "includeBhava", MakeBool(env, cfg.bindus_config.include_bhava != 0));
    SetNamed(env, bindus_cfg, "upagrahaConfig", WriteTimeUpagrahaConfig(env, cfg.bindus_config.upagraha_config));
    SetNamed(env, obj, "bindusConfig", bindus_cfg);

    napi_value drishti_cfg;
    napi_create_object(env, &drishti_cfg);
    SetNamed(env, drishti_cfg, "includeBhava", MakeBool(env, cfg.drishti_config.include_bhava != 0));
    SetNamed(env, drishti_cfg, "includeLagna", MakeBool(env, cfg.drishti_config.include_lagna != 0));
    SetNamed(env, drishti_cfg, "includeBindus", MakeBool(env, cfg.drishti_config.include_bindus != 0));
    SetNamed(env, obj, "drishtiConfig", drishti_cfg);

    napi_value amsha_scope;
    napi_create_object(env, &amsha_scope);
    SetNamed(env, amsha_scope, "includeBhavaCusps", MakeBool(env, cfg.amsha_scope.include_bhava_cusps != 0));
    SetNamed(env, amsha_scope, "includeArudhaPadas", MakeBool(env, cfg.amsha_scope.include_arudha_padas != 0));
    SetNamed(env, amsha_scope, "includeUpagrahas", MakeBool(env, cfg.amsha_scope.include_upagrahas != 0));
    SetNamed(env, amsha_scope, "includeSphutas", MakeBool(env, cfg.amsha_scope.include_sphutas != 0));
    SetNamed(env, amsha_scope, "includeSpecialLagnas", MakeBool(env, cfg.amsha_scope.include_special_lagnas != 0));
    SetNamed(env, obj, "amshaScope", amsha_scope);

    napi_value amsha_selection;
    napi_create_object(env, &amsha_selection);
    SetNamed(env, amsha_selection, "count", MakeUint32(env, cfg.amsha_selection.count));
    napi_value amsha_codes;
    napi_create_array_with_length(env, DHRUV_MAX_AMSHA_REQUESTS, &amsha_codes);
    napi_value amsha_variations;
    napi_create_array_with_length(env, DHRUV_MAX_AMSHA_REQUESTS, &amsha_variations);
    for (uint32_t i = 0; i < DHRUV_MAX_AMSHA_REQUESTS; ++i) {
        napi_set_element(env, amsha_codes, i, MakeUint32(env, cfg.amsha_selection.codes[i]));
        napi_set_element(env, amsha_variations, i, MakeUint32(env, cfg.amsha_selection.variations[i]));
    }
    SetNamed(env, amsha_selection, "codes", amsha_codes);
    SetNamed(env, amsha_selection, "variations", amsha_variations);
    SetNamed(env, obj, "amshaSelection", amsha_selection);

    SetNamed(env, obj, "includePanchang", MakeBool(env, cfg.include_panchang != 0));
    SetNamed(env, obj, "includeCalendar", MakeBool(env, cfg.include_calendar != 0));
    SetNamed(env, obj, "includeDasha", MakeBool(env, cfg.include_dasha != 0));

    napi_value dasha_cfg;
    napi_create_object(env, &dasha_cfg);
    SetNamed(env, dasha_cfg, "count", MakeUint32(env, cfg.dasha_config.count));
    napi_value systems;
    napi_create_array_with_length(env, DHRUV_MAX_DASHA_SYSTEMS, &systems);
    napi_value max_levels;
    napi_create_array_with_length(env, DHRUV_MAX_DASHA_SYSTEMS, &max_levels);
    for (uint32_t i = 0; i < DHRUV_MAX_DASHA_SYSTEMS; ++i) {
        napi_set_element(env, systems, i, MakeUint32(env, cfg.dasha_config.systems[i]));
        napi_set_element(env, max_levels, i, MakeUint32(env, cfg.dasha_config.max_levels[i]));
    }
    SetNamed(env, dasha_cfg, "systems", systems);
    SetNamed(env, dasha_cfg, "maxLevels", max_levels);
    SetNamed(env, dasha_cfg, "maxLevel", MakeUint32(env, cfg.dasha_config.max_level));
    napi_value methods;
    napi_create_array_with_length(env, 5, &methods);
    for (uint32_t i = 0; i < 5; ++i) {
        napi_set_element(env, methods, i, MakeUint32(env, cfg.dasha_config.level_methods[i]));
    }
    SetNamed(env, dasha_cfg, "levelMethods", methods);
    SetNamed(env, dasha_cfg, "yoginiScheme", MakeUint32(env, cfg.dasha_config.yogini_scheme));
    SetNamed(env, dasha_cfg, "useAbhijit", MakeBool(env, cfg.dasha_config.use_abhijit != 0));
    SetNamed(env, dasha_cfg, "hasSnapshotJd", MakeBool(env, cfg.dasha_config.has_snapshot_jd != 0));
    SetNamed(env, dasha_cfg, "snapshotJd", MakeDouble(env, cfg.dasha_config.snapshot_jd));
    SetNamed(env, obj, "dashaConfig", dasha_cfg);
    return obj;
}

napi_value DashaSelectionConfigDefault(napi_env env, napi_callback_info info) {
    (void)info;
    DhruvDashaSelectionConfig cfg = dhruv_dasha_selection_config_default();
    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "count", MakeUint32(env, cfg.count));

    napi_value systems;
    napi_create_array_with_length(env, DHRUV_MAX_DASHA_SYSTEMS, &systems);
    for (uint32_t i = 0; i < DHRUV_MAX_DASHA_SYSTEMS; ++i) {
        napi_set_element(env, systems, i, MakeUint32(env, cfg.systems[i]));
    }
    SetNamed(env, obj, "systems", systems);

    napi_value max_levels;
    napi_create_array_with_length(env, DHRUV_MAX_DASHA_SYSTEMS, &max_levels);
    for (uint32_t i = 0; i < DHRUV_MAX_DASHA_SYSTEMS; ++i) {
        napi_set_element(env, max_levels, i, MakeUint32(env, cfg.max_levels[i]));
    }
    SetNamed(env, obj, "maxLevels", max_levels);

    SetNamed(env, obj, "maxLevel", MakeUint32(env, cfg.max_level));
    napi_value methods;
    napi_create_array_with_length(env, 5, &methods);
    for (uint32_t i = 0; i < 5; ++i) {
        napi_set_element(env, methods, i, MakeUint32(env, cfg.level_methods[i]));
    }
    SetNamed(env, obj, "levelMethods", methods);
    SetNamed(env, obj, "yoginiScheme", MakeUint32(env, cfg.yogini_scheme));
    SetNamed(env, obj, "useAbhijit", MakeBool(env, cfg.use_abhijit != 0));
    SetNamed(env, obj, "hasSnapshotJd", MakeBool(env, cfg.has_snapshot_jd != 0));
    SetNamed(env, obj, "snapshotJd", MakeDouble(env, cfg.snapshot_jd));
    return obj;
}

napi_value DashaVariationConfigDefault(napi_env env, napi_callback_info info) {
    (void)info;
    return WriteDashaVariationConfig(env, dhruv_dasha_variation_config_default());
}

napi_value FullKundaliForDate(napi_env env, napi_callback_info info) {
    size_t argc = 9;
    napi_value args[9];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 9) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvUtcTime utc{};
    DhruvGeoLocation loc{};
    DhruvBhavaConfig bhava_cfg{};
    DhruvRiseSetConfig rise_cfg{};
    DhruvFullKundaliConfig full_cfg{};
    if (!ReadUtcTime(env, args[2], &utc) ||
        !ReadGeoLocation(env, args[3], &loc) ||
        !ReadBhavaConfig(env, args[4], &bhava_cfg) ||
        !ReadRiseSetConfig(env, args[5], &rise_cfg) ||
        !ReadFullKundaliConfig(env, args[8], &full_cfg)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    uint32_t ayanamsha = 0;
    bool use_nutation = false;
    if (!GetUint32(env, args[6], &ayanamsha) || !GetBool(env, args[7], &use_nutation)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvFullKundaliResult result{};
    int32_t status = dhruv_full_kundali_for_date(
        static_cast<const DhruvEngineHandle*>(e_ptr),
        static_cast<const DhruvEopHandle*>(ep_ptr),
        &utc,
        &loc,
        &bhava_cfg,
        &rise_cfg,
        ayanamsha,
        use_nutation ? 1 : 0,
        &full_cfg,
        &result);

    napi_value out = MakeStatusResult(env, status);
    if (status != STATUS_OK) {
        return out;
    }

    napi_value obj;
    napi_create_object(env, &obj);
    SetNamed(env, obj, "ayanamshaDeg", MakeDouble(env, result.ayanamsha_deg));
    if (result.bhava_cusps_valid != 0) SetNamed(env, obj, "bhavaCusps", WriteBhavaResult(env, result.bhava_cusps));
    if (result.graha_positions_valid != 0) SetNamed(env, obj, "grahaPositions", WriteGrahaPositions(env, result.graha_positions));
    if (result.bindus_valid != 0) SetNamed(env, obj, "bindus", WriteBindusResult(env, result.bindus));
    if (result.drishti_valid != 0) SetNamed(env, obj, "drishti", WriteDrishtiResult(env, result.drishti));
    if (result.ashtakavarga_valid != 0) SetNamed(env, obj, "ashtakavarga", WriteAshtakavargaResult(env, result.ashtakavarga));
    if (result.upagrahas_valid != 0) SetNamed(env, obj, "upagrahas", WriteAllUpagrahas(env, result.upagrahas));
    if (result.sphutas_valid != 0) SetNamed(env, obj, "sphutas", WriteSphutalResult(env, result.sphutas));
    if (result.special_lagnas_valid != 0) SetNamed(env, obj, "specialLagnas", WriteSpecialLagnas(env, result.special_lagnas));
    if (result.amshas_valid != 0) {
        napi_value amshas;
        napi_create_array_with_length(env, result.amshas_count, &amshas);
        for (uint32_t i = 0; i < result.amshas_count; ++i) {
            napi_set_element(env, amshas, i, WriteAmshaChart(env, result.amshas[i]));
        }
        SetNamed(env, obj, "amshas", amshas);
    }
    if (result.shadbala_valid != 0) SetNamed(env, obj, "shadbala", WriteShadbalaResult(env, result.shadbala));
    if (result.bhavabala_valid != 0) SetNamed(env, obj, "bhavabala", WriteBhavaBalaResult(env, result.bhavabala));
    if (result.vimsopaka_valid != 0) SetNamed(env, obj, "vimsopaka", WriteVimsopakaResult(env, result.vimsopaka));
    if (result.avastha_valid != 0) SetNamed(env, obj, "avastha", WriteAllGrahaAvasthas(env, result.avastha));
    if (result.charakaraka_valid != 0) SetNamed(env, obj, "charakaraka", WriteCharakarakaResult(env, result.charakaraka));
    if (result.panchang_valid != 0) SetNamed(env, obj, "panchang", WriteFullPanchangInfo(env, result.panchang));

    if (result.dasha_count > 0) {
        napi_value dashas;
        napi_create_array_with_length(env, result.dasha_count, &dashas);
        for (uint32_t i = 0; i < result.dasha_count; ++i) {
            napi_value hierarchy;
            status = WriteDashaHierarchyFromHandle(
                env,
                result.dasha_handles[i],
                result.dasha_systems[i],
                &hierarchy);
            if (status != STATUS_OK) {
                dhruv_full_kundali_result_free(&result);
                return MakeStatusResult(env, status);
            }
            napi_set_element(env, dashas, i, hierarchy);
        }
        SetNamed(env, obj, "dasha", dashas);
    }

    if (result.dasha_snapshot_count > 0) {
        napi_value snapshots;
        napi_create_array_with_length(env, result.dasha_snapshot_count, &snapshots);
        for (uint32_t i = 0; i < result.dasha_snapshot_count; ++i) {
            const DhruvDashaSnapshot& snapshot = result.dasha_snapshots[i];
            napi_value so;
            napi_create_object(env, &so);
            SetNamed(env, so, "system", MakeUint32(env, snapshot.system));
            SetNamed(env, so, "queryJd", MakeDouble(env, snapshot.query_jd));
            SetNamed(env, so, "count", MakeUint32(env, snapshot.count));
            napi_value periods;
            napi_create_array_with_length(env, snapshot.count, &periods);
            for (uint32_t j = 0; j < snapshot.count && j < 5; ++j) {
                napi_set_element(env, periods, j, WriteDashaPeriod(env, snapshot.periods[j]));
            }
            SetNamed(env, so, "periods", periods);
            napi_set_element(env, snapshots, i, so);
        }
        SetNamed(env, obj, "dashaSnapshots", snapshots);
    }

    dhruv_full_kundali_result_free(&result);
    SetNamed(env, out, "result", obj);
    return out;
}

napi_value DashaHierarchyUtc(napi_env env, napi_callback_info info) {
    size_t argc = 8;
    napi_value args[8];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 8) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvUtcTime birth{};
    DhruvGeoLocation loc{};
    if (!ReadUtcTime(env, args[2], &birth) || !ReadGeoLocation(env, args[3], &loc)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    uint32_t ayanamsha = 0;
    bool use_nutation = false;
    uint32_t system = 0;
    uint32_t max_level = 0;
    if (!GetUint32(env, args[4], &ayanamsha) || !GetBool(env, args[5], &use_nutation) || !GetUint32(env, args[6], &system) || !GetUint32(env, args[7], &max_level)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvBhavaConfig bhava_cfg = dhruv_bhava_config_default();
    DhruvRiseSetConfig rise_cfg = dhruv_riseset_config_default();
    DhruvDashaHierarchyHandle handle = nullptr;

    int32_t status = dhruv_dasha_hierarchy_utc(
        static_cast<const DhruvEngineHandle*>(e_ptr),
        static_cast<const DhruvEopHandle*>(ep_ptr),
        &birth,
        &loc,
        &bhava_cfg,
        &rise_cfg,
        ayanamsha,
        use_nutation ? 1 : 0,
        static_cast<uint8_t>(system),
        static_cast<uint8_t>(max_level),
        &handle);

    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK && handle != nullptr) {
        SetNamed(env, out, "handle", MakeExternalPtr(env, handle));
    } else {
        napi_value nullv;
        napi_get_null(env, &nullv);
        SetNamed(env, out, "handle", nullv);
    }
    return out;
}

napi_value DashaHierarchyFree(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value args[1];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 1) {
        napi_value undef;
        napi_get_undefined(env, &undef);
        return undef;
    }

    void* ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &ptr)) {
        napi_value undef;
        napi_get_undefined(env, &undef);
        return undef;
    }

    dhruv_dasha_hierarchy_free(reinterpret_cast<DhruvDashaHierarchyHandle>(ptr));
    napi_value undef;
    napi_get_undefined(env, &undef);
    return undef;
}

napi_value DashaHierarchyLevelCount(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value args[1];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 1) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    void* ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    uint8_t count = 0;
    int32_t status = dhruv_dasha_hierarchy_level_count(reinterpret_cast<DhruvDashaHierarchyHandle>(ptr), &count);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "count", MakeUint32(env, count));
    }
    return out;
}

napi_value DashaHierarchyPeriodCount(napi_env env, napi_callback_info info) {
    size_t argc = 2;
    napi_value args[2];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 2) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    void* ptr = nullptr;
    uint32_t level = 0;
    if (!ReadExternalPtr(env, args[0], &ptr) || !GetUint32(env, args[1], &level)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    uint32_t count = 0;
    int32_t status = dhruv_dasha_hierarchy_period_count(
        reinterpret_cast<DhruvDashaHierarchyHandle>(ptr),
        static_cast<uint8_t>(level),
        &count);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "count", MakeUint32(env, count));
    }
    return out;
}

napi_value DashaHierarchyPeriodAt(napi_env env, napi_callback_info info) {
    size_t argc = 3;
    napi_value args[3];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 3) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    void* ptr = nullptr;
    uint32_t level = 0;
    uint32_t idx = 0;
    if (!ReadExternalPtr(env, args[0], &ptr) || !GetUint32(env, args[1], &level) || !GetUint32(env, args[2], &idx)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvDashaPeriod period{};
    int32_t status = dhruv_dasha_hierarchy_period_at(
        reinterpret_cast<DhruvDashaHierarchyHandle>(ptr),
        static_cast<uint8_t>(level),
        idx,
        &period);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "period", WriteDashaPeriod(env, period));
    }
    return out;
}

napi_value DashaSnapshotUtc(napi_env env, napi_callback_info info) {
    size_t argc = 9;
    napi_value args[9];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 9) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvUtcTime birth{};
    DhruvUtcTime query{};
    DhruvGeoLocation loc{};
    if (!ReadUtcTime(env, args[2], &birth) || !ReadUtcTime(env, args[3], &query) || !ReadGeoLocation(env, args[4], &loc)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    uint32_t ayanamsha = 0;
    bool use_nutation = false;
    uint32_t system = 0;
    uint32_t max_level = 0;
    if (!GetUint32(env, args[5], &ayanamsha) || !GetBool(env, args[6], &use_nutation) || !GetUint32(env, args[7], &system) || !GetUint32(env, args[8], &max_level)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvBhavaConfig bhava_cfg = dhruv_bhava_config_default();
    DhruvRiseSetConfig rise_cfg = dhruv_riseset_config_default();
    DhruvDashaSnapshot snapshot{};

    int32_t status = dhruv_dasha_snapshot_utc(
        static_cast<const DhruvEngineHandle*>(e_ptr),
        static_cast<const DhruvEopHandle*>(ep_ptr),
        &birth,
        &query,
        &loc,
        &bhava_cfg,
        &rise_cfg,
        ayanamsha,
        use_nutation ? 1 : 0,
        static_cast<uint8_t>(system),
        static_cast<uint8_t>(max_level),
        &snapshot);

    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        napi_value snap;
        napi_create_object(env, &snap);
        SetNamed(env, snap, "system", MakeUint32(env, snapshot.system));
        SetNamed(env, snap, "queryJd", MakeDouble(env, snapshot.query_jd));
        SetNamed(env, snap, "count", MakeUint32(env, snapshot.count));

        napi_value periods;
        napi_create_array_with_length(env, snapshot.count, &periods);
        for (uint32_t i = 0; i < snapshot.count; ++i) {
            napi_set_element(env, periods, i, WriteDashaPeriod(env, snapshot.periods[i]));
        }
        SetNamed(env, snap, "periods", periods);
        SetNamed(env, out, "snapshot", snap);
    }

    return out;
}

napi_value DashaLevel0Utc(napi_env env, napi_callback_info info) {
    size_t argc = 7;
    napi_value args[7];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 7) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvUtcTime birth{};
    DhruvGeoLocation loc{};
    if (!ReadUtcTime(env, args[2], &birth) || !ReadGeoLocation(env, args[3], &loc)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    uint32_t ayanamsha = 0;
    bool use_nutation = false;
    uint32_t system = 0;
    if (!GetUint32(env, args[4], &ayanamsha) || !GetBool(env, args[5], &use_nutation) || !GetUint32(env, args[6], &system)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvBhavaConfig bhava_cfg = dhruv_bhava_config_default();
    DhruvRiseSetConfig rise_cfg = dhruv_riseset_config_default();
    DhruvDashaPeriodListHandle handle = nullptr;
    int32_t status = dhruv_dasha_level0_utc(
        static_cast<const DhruvEngineHandle*>(e_ptr),
        static_cast<const DhruvEopHandle*>(ep_ptr),
        &birth,
        &loc,
        &bhava_cfg,
        &rise_cfg,
        ayanamsha,
        use_nutation ? 1 : 0,
        static_cast<uint8_t>(system),
        &handle);

    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        napi_value periods;
        status = WriteDashaPeriodList(env, handle, &periods);
        SetNamed(env, out, "status", MakeInt32(env, status));
        if (status == STATUS_OK) {
            SetNamed(env, out, "periods", periods);
        }
    }
    return out;
}

napi_value DashaLevel0EntityUtc(napi_env env, napi_callback_info info) {
    size_t argc = 9;
    napi_value args[9];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 9) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvUtcTime birth{};
    DhruvGeoLocation loc{};
    if (!ReadUtcTime(env, args[2], &birth) || !ReadGeoLocation(env, args[3], &loc)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    uint32_t ayanamsha = 0;
    bool use_nutation = false;
    uint32_t system = 0;
    uint32_t entity_type = 0;
    uint32_t entity_index = 0;
    if (!GetUint32(env, args[4], &ayanamsha) || !GetBool(env, args[5], &use_nutation) ||
        !GetUint32(env, args[6], &system) || !GetUint32(env, args[7], &entity_type) ||
        !GetUint32(env, args[8], &entity_index)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvBhavaConfig bhava_cfg = dhruv_bhava_config_default();
    DhruvRiseSetConfig rise_cfg = dhruv_riseset_config_default();
    uint8_t found = 0;
    DhruvDashaPeriod period{};
    int32_t status = dhruv_dasha_level0_entity_utc(
        static_cast<const DhruvEngineHandle*>(e_ptr),
        static_cast<const DhruvEopHandle*>(ep_ptr),
        &birth,
        &loc,
        &bhava_cfg,
        &rise_cfg,
        ayanamsha,
        use_nutation ? 1 : 0,
        static_cast<uint8_t>(system),
        static_cast<uint8_t>(entity_type),
        static_cast<uint8_t>(entity_index),
        &found,
        &period);

    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "found", MakeBool(env, found != 0));
        if (found != 0) {
            SetNamed(env, out, "period", WriteDashaPeriod(env, period));
        }
    }
    return out;
}

napi_value DashaChildrenUtc(napi_env env, napi_callback_info info) {
    size_t argc = 9;
    napi_value args[9];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 9) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvUtcTime birth{};
    DhruvGeoLocation loc{};
    if (!ReadUtcTime(env, args[2], &birth) || !ReadGeoLocation(env, args[3], &loc)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    uint32_t ayanamsha = 0;
    bool use_nutation = false;
    uint32_t system = 0;
    if (!GetUint32(env, args[4], &ayanamsha) || !GetBool(env, args[5], &use_nutation) || !GetUint32(env, args[6], &system)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvDashaVariationConfig variation_cfg = dhruv_dasha_variation_config_default();
    napi_valuetype variation_type = napi_undefined;
    if (napi_typeof(env, args[7], &variation_type) != napi_ok) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }
    if (variation_type != napi_null && variation_type != napi_undefined &&
        !ReadDashaVariationConfig(env, args[7], &variation_cfg)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvDashaPeriod parent{};
    if (!ReadDashaPeriodValue(env, args[8], &parent)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvBhavaConfig bhava_cfg = dhruv_bhava_config_default();
    DhruvRiseSetConfig rise_cfg = dhruv_riseset_config_default();
    DhruvDashaPeriodListHandle handle = nullptr;
    int32_t status = dhruv_dasha_children_utc(
        static_cast<const DhruvEngineHandle*>(e_ptr),
        static_cast<const DhruvEopHandle*>(ep_ptr),
        &birth,
        &loc,
        &bhava_cfg,
        &rise_cfg,
        ayanamsha,
        use_nutation ? 1 : 0,
        static_cast<uint8_t>(system),
        &variation_cfg,
        &parent,
        &handle);

    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        napi_value periods;
        status = WriteDashaPeriodList(env, handle, &periods);
        SetNamed(env, out, "status", MakeInt32(env, status));
        if (status == STATUS_OK) {
            SetNamed(env, out, "periods", periods);
        }
    }
    return out;
}

napi_value DashaChildPeriodUtc(napi_env env, napi_callback_info info) {
    size_t argc = 11;
    napi_value args[11];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 11) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvUtcTime birth{};
    DhruvGeoLocation loc{};
    if (!ReadUtcTime(env, args[2], &birth) || !ReadGeoLocation(env, args[3], &loc)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    uint32_t ayanamsha = 0;
    bool use_nutation = false;
    uint32_t system = 0;
    if (!GetUint32(env, args[4], &ayanamsha) || !GetBool(env, args[5], &use_nutation) || !GetUint32(env, args[6], &system)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvDashaVariationConfig variation_cfg = dhruv_dasha_variation_config_default();
    napi_valuetype variation_type = napi_undefined;
    if (napi_typeof(env, args[7], &variation_type) != napi_ok) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }
    if (variation_type != napi_null && variation_type != napi_undefined &&
        !ReadDashaVariationConfig(env, args[7], &variation_cfg)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvDashaPeriod parent{};
    uint32_t child_entity_type = 0;
    uint32_t child_entity_index = 0;
    if (!ReadDashaPeriodValue(env, args[8], &parent) || !GetUint32(env, args[9], &child_entity_type) ||
        !GetUint32(env, args[10], &child_entity_index)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvBhavaConfig bhava_cfg = dhruv_bhava_config_default();
    DhruvRiseSetConfig rise_cfg = dhruv_riseset_config_default();
    uint8_t found = 0;
    DhruvDashaPeriod period{};
    int32_t status = dhruv_dasha_child_period_utc(
        static_cast<const DhruvEngineHandle*>(e_ptr),
        static_cast<const DhruvEopHandle*>(ep_ptr),
        &birth,
        &loc,
        &bhava_cfg,
        &rise_cfg,
        ayanamsha,
        use_nutation ? 1 : 0,
        static_cast<uint8_t>(system),
        &variation_cfg,
        &parent,
        static_cast<uint8_t>(child_entity_type),
        static_cast<uint8_t>(child_entity_index),
        &found,
        &period);

    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "found", MakeBool(env, found != 0));
        if (found != 0) {
            SetNamed(env, out, "period", WriteDashaPeriod(env, period));
        }
    }
    return out;
}

napi_value DashaCompleteLevelUtc(napi_env env, napi_callback_info info) {
    size_t argc = 10;
    napi_value args[10];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 10) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvUtcTime birth{};
    DhruvGeoLocation loc{};
    if (!ReadUtcTime(env, args[2], &birth) || !ReadGeoLocation(env, args[3], &loc)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    uint32_t ayanamsha = 0;
    bool use_nutation = false;
    uint32_t system = 0;
    if (!GetUint32(env, args[4], &ayanamsha) || !GetBool(env, args[5], &use_nutation) || !GetUint32(env, args[6], &system)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvDashaVariationConfig variation_cfg = dhruv_dasha_variation_config_default();
    napi_valuetype variation_type = napi_undefined;
    if (napi_typeof(env, args[7], &variation_type) != napi_ok) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }
    if (variation_type != napi_null && variation_type != napi_undefined &&
        !ReadDashaVariationConfig(env, args[7], &variation_cfg)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    bool is_array = false;
    if (napi_is_array(env, args[8], &is_array) != napi_ok || !is_array) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }
    uint32_t parent_count = 0;
    if (napi_get_array_length(env, args[8], &parent_count) != napi_ok) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }
    std::vector<DhruvDashaPeriod> parent_periods(parent_count);
    for (uint32_t i = 0; i < parent_count; ++i) {
        napi_value item;
        if (napi_get_element(env, args[8], i, &item) != napi_ok || !ReadDashaPeriodValue(env, item, &parent_periods[i])) {
            return MakeStatusResult(env, STATUS_INVALID_INPUT);
        }
    }

    uint32_t child_level = 0;
    if (!GetUint32(env, args[9], &child_level)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvBhavaConfig bhava_cfg = dhruv_bhava_config_default();
    DhruvRiseSetConfig rise_cfg = dhruv_riseset_config_default();
    DhruvDashaPeriodListHandle handle = nullptr;
    int32_t status = dhruv_dasha_complete_level_utc(
        static_cast<const DhruvEngineHandle*>(e_ptr),
        static_cast<const DhruvEopHandle*>(ep_ptr),
        &birth,
        &loc,
        &bhava_cfg,
        &rise_cfg,
        ayanamsha,
        use_nutation ? 1 : 0,
        static_cast<uint8_t>(system),
        &variation_cfg,
        parent_periods.empty() ? nullptr : parent_periods.data(),
        parent_count,
        static_cast<uint8_t>(child_level),
        &handle);

    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        napi_value periods;
        status = WriteDashaPeriodList(env, handle, &periods);
        SetNamed(env, out, "status", MakeInt32(env, status));
        if (status == STATUS_OK) {
            SetNamed(env, out, "periods", periods);
        }
    }
    return out;
}

napi_value TaraCatalogLoad(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value args[1];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 1) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    std::string path;
    if (!GetString(env, args[0], &path)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvTaraCatalogHandle* handle = nullptr;
    int32_t status = dhruv_tara_catalog_load(reinterpret_cast<const uint8_t*>(path.data()), static_cast<uint32_t>(path.size()), &handle);

    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK && handle != nullptr) {
        SetNamed(env, out, "handle", MakeExternalPtr(env, handle));
    } else {
        napi_value nullv;
        napi_get_null(env, &nullv);
        SetNamed(env, out, "handle", nullv);
    }
    return out;
}

napi_value TaraCatalogFree(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value args[1];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 1) {
        napi_value undef;
        napi_get_undefined(env, &undef);
        return undef;
    }

    void* ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &ptr)) {
        napi_value undef;
        napi_get_undefined(env, &undef);
        return undef;
    }

    dhruv_tara_catalog_free(static_cast<DhruvTaraCatalogHandle*>(ptr));
    napi_value undef;
    napi_get_undefined(env, &undef);
    return undef;
}

napi_value TaraGalacticCenterEcliptic(napi_env env, napi_callback_info info) {
    size_t argc = 2;
    napi_value args[2];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 2) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    void* ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &ptr)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    double jd = 0.0;
    if (!GetDouble(env, args[1], &jd)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvSphericalCoords coords{};
    int32_t status = dhruv_tara_galactic_center_ecliptic(static_cast<const DhruvTaraCatalogHandle*>(ptr), jd, &coords);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        napi_value c;
        napi_create_object(env, &c);
        SetNamed(env, c, "lonDeg", MakeDouble(env, coords.lon_deg));
        SetNamed(env, c, "latDeg", MakeDouble(env, coords.lat_deg));
        SetNamed(env, c, "distanceKm", MakeDouble(env, coords.distance_km));
        SetNamed(env, out, "coords", c);
    }
    return out;
}

napi_value TaraComputeEx(napi_env env, napi_callback_info info) {
    size_t argc = 2;
    napi_value args[2];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 2) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    void* ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &ptr)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvTaraComputeRequest req{};
    napi_value v;
    if (!GetNamedProperty(env, args[1], "taraId", &v) || !GetInt32(env, v, &req.tara_id)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "outputKind", &v) || !GetInt32(env, v, &req.output_kind)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "jdTdb", &v) || !GetDouble(env, v, &req.jd_tdb)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    bool has_ayan = false;
    if (!GetOptionalNamedProperty(env, args[1], "ayanamshaDeg", &v, &has_ayan)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    req.ayanamsha_deg = 0.0;
    if (has_ayan && !GetDouble(env, v, &req.ayanamsha_deg)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    req.config.accuracy = 0;
    req.config.apply_parallax = 1;

    bool has_cfg = false;
    napi_value cfg_obj;
    if (!GetOptionalNamedProperty(env, args[1], "config", &cfg_obj, &has_cfg)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (has_cfg) {
        bool has_acc = false;
        if (!GetOptionalNamedProperty(env, cfg_obj, "accuracy", &v, &has_acc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        if (has_acc && !GetInt32(env, v, &req.config.accuracy)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

        bool has_par = false;
        if (!GetOptionalNamedProperty(env, cfg_obj, "applyParallax", &v, &has_par)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        if (has_par) {
            bool b = true;
            if (!GetBool(env, v, &b)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
            req.config.apply_parallax = b ? 1 : 0;
        }
    }

    req.earth_state_valid = 0;
    bool has_earth = false;
    napi_value earth_obj;
    if (!GetOptionalNamedProperty(env, args[1], "earthState", &earth_obj, &has_earth)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (has_earth) {
        req.earth_state_valid = 1;
        napi_value pos, vel;
        bool pos_ok = GetNamedProperty(env, earth_obj, "positionAu", &pos);
        bool vel_ok = GetNamedProperty(env, earth_obj, "velocityAuDay", &vel);
        if (!pos_ok || !vel_ok) return MakeStatusResult(env, STATUS_INVALID_INPUT);

        bool is_arr = false;
        napi_is_array(env, pos, &is_arr);
        if (!is_arr) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        napi_is_array(env, vel, &is_arr);
        if (!is_arr) return MakeStatusResult(env, STATUS_INVALID_INPUT);

        for (uint32_t i = 0; i < 3; ++i) {
            napi_value e;
            napi_get_element(env, pos, i, &e);
            if (!GetDouble(env, e, &req.earth_state.position_au[i])) return MakeStatusResult(env, STATUS_INVALID_INPUT);
            napi_get_element(env, vel, i, &e);
            if (!GetDouble(env, e, &req.earth_state.velocity_au_day[i])) return MakeStatusResult(env, STATUS_INVALID_INPUT);
        }
    }

    DhruvTaraComputeResult out_val{};
    int32_t status = dhruv_tara_compute_ex(static_cast<const DhruvTaraCatalogHandle*>(ptr), &req, &out_val);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        napi_value res;
        napi_create_object(env, &res);
        SetNamed(env, res, "outputKind", MakeInt32(env, out_val.output_kind));

        napi_value eq;
        napi_create_object(env, &eq);
        SetNamed(env, eq, "raDeg", MakeDouble(env, out_val.equatorial.ra_deg));
        SetNamed(env, eq, "decDeg", MakeDouble(env, out_val.equatorial.dec_deg));
        SetNamed(env, eq, "distanceAu", MakeDouble(env, out_val.equatorial.distance_au));
        SetNamed(env, res, "equatorial", eq);

        napi_value ecl;
        napi_create_object(env, &ecl);
        SetNamed(env, ecl, "lonDeg", MakeDouble(env, out_val.ecliptic.lon_deg));
        SetNamed(env, ecl, "latDeg", MakeDouble(env, out_val.ecliptic.lat_deg));
        SetNamed(env, ecl, "distanceKm", MakeDouble(env, out_val.ecliptic.distance_km));
        SetNamed(env, res, "ecliptic", ecl);

        SetNamed(env, res, "siderealLongitudeDeg", MakeDouble(env, out_val.sidereal_longitude_deg));
        SetNamed(env, out, "result", res);
    }

    return out;
}

napi_value Init(napi_env env, napi_value exports) {
    napi_property_descriptor props[] = {
        {"apiVersion", nullptr, ApiVersion, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"configLoad", nullptr, ConfigLoad, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"configFree", nullptr, ConfigFree, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"configClearActive", nullptr, ConfigClearActive, nullptr, nullptr, nullptr, napi_default, nullptr},

        {"engineNew", nullptr, EngineNew, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"engineFree", nullptr, EngineFree, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"engineQuery", nullptr, EngineQuery, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"queryOnce", nullptr, QueryOnce, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"cartesianToSpherical", nullptr, CartesianToSpherical, nullptr, nullptr, nullptr, napi_default, nullptr},

        {"lskLoad", nullptr, LskLoad, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"lskFree", nullptr, LskFree, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"eopLoad", nullptr, EopLoad, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"eopFree", nullptr, EopFree, nullptr, nullptr, nullptr, napi_default, nullptr},

        {"utcToTdbJd", nullptr, UtcToTdbJd, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"jdTdbToUtc", nullptr, JdTdbToUtc, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"nutationIau2000b", nullptr, NutationIau2000b, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"nutationIau2000bUtc", nullptr, NutationIau2000bUtc, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"approximateLocalNoonJd", nullptr, ApproximateLocalNoonJd, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"ayanamshaSystemCount", nullptr, AyanamshaSystemCount, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"referencePlaneDefault", nullptr, ReferencePlaneDefault, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"ayanamshaComputeEx", nullptr, AyanamshaComputeEx, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"lunarNodeCount", nullptr, LunarNodeCount, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"lunarNodeDeg", nullptr, LunarNodeDeg, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"lunarNodeDegWithEngine", nullptr, LunarNodeDegWithEngine, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"lunarNodeDegUtc", nullptr, LunarNodeDegUtc, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"lunarNodeDegUtcWithEngine", nullptr, LunarNodeDegUtcWithEngine, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"lunarNodeComputeEx", nullptr, LunarNodeComputeEx, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"rashiCount", nullptr, RashiCount, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"nakshatraCount", nullptr, NakshatraCount, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"rashiFromLongitude", nullptr, RashiFromLongitude, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"nakshatraFromLongitude", nullptr, NakshatraFromLongitude, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"nakshatra28FromLongitude", nullptr, Nakshatra28FromLongitude, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"rashiFromTropical", nullptr, RashiFromTropical, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"nakshatraFromTropical", nullptr, NakshatraFromTropical, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"nakshatra28FromTropical", nullptr, Nakshatra28FromTropical, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"rashiFromTropicalUtc", nullptr, RashiFromTropicalUtc, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"nakshatraFromTropicalUtc", nullptr, NakshatraFromTropicalUtc, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"nakshatra28FromTropicalUtc", nullptr, Nakshatra28FromTropicalUtc, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"grahaTropicalLongitudes", nullptr, GrahaTropicalLongitudes, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"rashiName", nullptr, RashiName, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"nakshatraName", nullptr, NakshatraName, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"nakshatra28Name", nullptr, Nakshatra28Name, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"masaName", nullptr, MasaName, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"ayanaName", nullptr, AyanaName, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"samvatsaraName", nullptr, SamvatsaraName, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"tithiName", nullptr, TithiName, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"karanaName", nullptr, KaranaName, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"yogaName", nullptr, YogaName, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"vaarName", nullptr, VaarName, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"horaName", nullptr, HoraName, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"grahaName", nullptr, GrahaName, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"yoginiName", nullptr, YoginiName, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"sphutaName", nullptr, SphutaName, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"specialLagnaName", nullptr, SpecialLagnaName, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"arudhaPadaName", nullptr, ArudhaPadaName, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"upagrahaName", nullptr, UpagrahaName, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"degToDms", nullptr, DegToDms, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"tithiFromElongation", nullptr, TithiFromElongation, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"karanaFromElongation", nullptr, KaranaFromElongation, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"yogaFromSum", nullptr, YogaFromSum, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"vaarFromJd", nullptr, VaarFromJd, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"masaFromRashiIndex", nullptr, MasaFromRashiIndex, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"ayanaFromSiderealLongitude", nullptr, AyanaFromSiderealLongitude, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"samvatsaraFromYear", nullptr, SamvatsaraFromYear, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"nthRashiFrom", nullptr, NthRashiFrom, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"rashiLord", nullptr, RashiLord, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"horaAt", nullptr, HoraAt, nullptr, nullptr, nullptr, napi_default, nullptr},

        {"riseSetConfigDefault", nullptr, RiseSetConfigDefault, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"bhavaConfigDefault", nullptr, BhavaConfigDefault, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"bhavaSystemCount", nullptr, BhavaSystemCount, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"sankrantiConfigDefault", nullptr, SankrantiConfigDefault, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"conjunctionConfigDefault", nullptr, ConjunctionConfigDefault, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"grahanConfigDefault", nullptr, GrahanConfigDefault, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"stationaryConfigDefault", nullptr, StationaryConfigDefault, nullptr, nullptr, nullptr, napi_default, nullptr},

        {"computeRiseSet", nullptr, ComputeRiseSet, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"computeAllEvents", nullptr, ComputeAllEvents, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"computeRiseSetUtc", nullptr, ComputeRiseSetUtc, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"computeAllEventsUtc", nullptr, ComputeAllEventsUtc, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"computeBhavas", nullptr, ComputeBhavas, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"computeBhavasUtc", nullptr, ComputeBhavasUtc, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"lagnaDeg", nullptr, LagnaDeg, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"mcDeg", nullptr, MCDeg, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"ramcDeg", nullptr, RAMCDeg, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"lagnaDegUtc", nullptr, LagnaDegUtc, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"mcDegUtc", nullptr, MCDegUtc, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"ramcDegUtc", nullptr, RAMCDegUtc, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"riseSetResultToUtc", nullptr, RiseSetResultToUtc, nullptr, nullptr, nullptr, napi_default, nullptr},

        {"conjunctionSearch", nullptr, ConjunctionSearch, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"grahanSearch", nullptr, GrahanSearch, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"motionSearch", nullptr, MotionSearch, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"lunarPhaseSearch", nullptr, LunarPhaseSearch, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"sankrantiSearch", nullptr, SankrantiSearch, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"tithiForDate", nullptr, TithiForDate, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"karanaForDate", nullptr, KaranaForDate, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"yogaForDate", nullptr, YogaForDate, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"nakshatraForDate", nullptr, NakshatraForDate, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"vaarForDate", nullptr, VaarForDate, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"horaForDate", nullptr, HoraForDate, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"ghatikaForDate", nullptr, GhatikaForDate, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"masaForDate", nullptr, MasaForDate, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"ayanaForDate", nullptr, AyanaForDate, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"varshaForDate", nullptr, VarshaForDate, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"panchangComputeEx", nullptr, PanchangComputeEx, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"elongationAt", nullptr, ElongationAt, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"siderealSumAt", nullptr, SiderealSumAt, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"vedicDaySunrises", nullptr, VedicDaySunrises, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"bodyEclipticLonLat", nullptr, BodyEclipticLonLat, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"tithiAt", nullptr, TithiAt, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"karanaAt", nullptr, KaranaAt, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"yogaAt", nullptr, YogaAt, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"vaarFromSunrises", nullptr, VaarFromSunrises, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"horaFromSunrises", nullptr, HoraFromSunrises, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"ghatikaFromSunrises", nullptr, GhatikaFromSunrises, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"nakshatraAt", nullptr, NakshatraAt, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"ghatikaFromElapsed", nullptr, GhatikaFromElapsed, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"ghatikasSinceSunrise", nullptr, GhatikasSinceSunrise, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"allSphutas", nullptr, AllSphutas, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"bhriguBindu", nullptr, BhriguBindu, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"pranaSphuta", nullptr, PranaSphuta, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"dehaSphuta", nullptr, DehaSphuta, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"mrityuSphuta", nullptr, MrityuSphuta, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"tithiSphuta", nullptr, TithiSphuta, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"yogaSphuta", nullptr, YogaSphuta, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"yogaSphutaNormalized", nullptr, YogaSphutaNormalized, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"rahuTithiSphuta", nullptr, RahuTithiSphuta, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"kshetraSphuta", nullptr, KshetraSphuta, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"beejaSphuta", nullptr, BeejaSphuta, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"trisphuta", nullptr, Trisphuta, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"chatussphuta", nullptr, Chatussphuta, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"panchasphuta", nullptr, Panchasphuta, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"sookshmaTrisphuta", nullptr, SookshmaTrisphuta, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"avayogaSphuta", nullptr, AvayogaSphuta, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"kunda", nullptr, Kunda, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"bhavaLagna", nullptr, BhavaLagna, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"horaLagna", nullptr, HoraLagna, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"ghatiLagna", nullptr, GhatiLagna, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"vighatiLagna", nullptr, VighatiLagna, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"varnadaLagna", nullptr, VarnadaLagna, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"sreeLagna", nullptr, SreeLagna, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"pranapadaLagna", nullptr, PranapadaLagna, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"induLagna", nullptr, InduLagna, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"arudhaPada", nullptr, ArudhaPada, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"sunBasedUpagrahas", nullptr, SunBasedUpagrahas, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"timeUpagrahaJd", nullptr, TimeUpagrahaJd, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"timeUpagrahaJdUtc", nullptr, TimeUpagrahaJdUtc, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"calculateAshtakavarga", nullptr, CalculateAshtakavarga, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"calculateBhavaBala", nullptr, CalculateBhavaBala, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"calculateBav", nullptr, CalculateBav, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"calculateAllBav", nullptr, CalculateAllBav, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"calculateSav", nullptr, CalculateSav, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"trikonaSodhana", nullptr, TrikonaSodhana, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"ekadhipatyaSodhana", nullptr, EkadhipatyaSodhana, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"ashtakavargaForDate", nullptr, AshtakavargaForDate, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"grahaDrishti", nullptr, GrahaDrishti, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"grahaDrishtiMatrix", nullptr, GrahaDrishtiMatrix, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"drishtiForDate", nullptr, DrishtiForDate, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"grahaPositionsForDate", nullptr, GrahaPositionsForDate, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"coreBindusForDate", nullptr, CoreBindusForDate, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"amshaLongitude", nullptr, AmshaLongitude, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"amshaRashiInfo", nullptr, AmshaRashiInfo, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"amshaLongitudes", nullptr, AmshaLongitudes, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"amshaChartForDate", nullptr, AmshaChartForDate, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"grahaSiderealLongitudes", nullptr, GrahaSiderealLongitudes, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"specialLagnasForDate", nullptr, SpecialLagnasForDate, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"arudhaPadasForDate", nullptr, ArudhaPadasForDate, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"allUpagrahasForDate", nullptr, AllUpagrahasForDate, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"shadbalaForDate", nullptr, ShadbalaForDate, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"bhavaBalaForDate", nullptr, BhavaBalaForDate, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"vimsopakaForDate", nullptr, VimsopakaForDate, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"balasForDate", nullptr, BalasForDate, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"avasthaForDate", nullptr, AvasthaForDate, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"charakarakaForDate", nullptr, CharakarakaForDate, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"fullKundaliSummaryForDate", nullptr, FullKundaliSummaryForDate, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"fullKundaliConfigDefault", nullptr, FullKundaliConfigDefault, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"fullKundaliForDate", nullptr, FullKundaliForDate, nullptr, nullptr, nullptr, napi_default, nullptr},

        {"dashaSelectionConfigDefault", nullptr, DashaSelectionConfigDefault, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"dashaVariationConfigDefault", nullptr, DashaVariationConfigDefault, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"dashaHierarchyUtc", nullptr, DashaHierarchyUtc, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"dashaHierarchyFree", nullptr, DashaHierarchyFree, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"dashaHierarchyLevelCount", nullptr, DashaHierarchyLevelCount, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"dashaHierarchyPeriodCount", nullptr, DashaHierarchyPeriodCount, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"dashaHierarchyPeriodAt", nullptr, DashaHierarchyPeriodAt, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"dashaSnapshotUtc", nullptr, DashaSnapshotUtc, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"dashaLevel0Utc", nullptr, DashaLevel0Utc, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"dashaLevel0EntityUtc", nullptr, DashaLevel0EntityUtc, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"dashaChildrenUtc", nullptr, DashaChildrenUtc, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"dashaChildPeriodUtc", nullptr, DashaChildPeriodUtc, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"dashaCompleteLevelUtc", nullptr, DashaCompleteLevelUtc, nullptr, nullptr, nullptr, napi_default, nullptr},

        {"taraCatalogLoad", nullptr, TaraCatalogLoad, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"taraCatalogFree", nullptr, TaraCatalogFree, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"taraGalacticCenterEcliptic", nullptr, TaraGalacticCenterEcliptic, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"taraComputeEx", nullptr, TaraComputeEx, nullptr, nullptr, nullptr, napi_default, nullptr},
    };

    napi_define_properties(env, exports, sizeof(props) / sizeof(props[0]), props);
    return exports;
}

}  // namespace

NAPI_MODULE(NODE_GYP_MODULE_NAME, Init)
