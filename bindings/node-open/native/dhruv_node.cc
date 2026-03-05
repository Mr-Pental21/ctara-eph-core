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

    DhruvQuery q{};
    napi_value v;
    if (!GetNamedProperty(env, args[1], "target", &v) || !GetInt32(env, v, &q.target)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "observer", &v) || !GetInt32(env, v, &q.observer)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "frame", &v) || !GetInt32(env, v, &q.frame)) return MakeStatusResult(env, STATUS_INVALID_INPUT);
    if (!GetNamedProperty(env, args[1], "epochTdbJd", &v) || !GetDouble(env, v, &q.epoch_tdb_jd)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvStateVector out_vec{};
    int32_t status = dhruv_engine_query(static_cast<const DhruvEngineHandle*>(ptr), &q, &out_vec);

    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "state", WriteStateVector(env, out_vec));
    }
    return out;
}

napi_value QueryUtcSpherical(napi_env env, napi_callback_info info) {
    size_t argc = 5;
    napi_value args[5];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 5) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    void* ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &ptr)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    int32_t target = 0;
    int32_t observer = 0;
    int32_t frame = 0;
    if (!GetInt32(env, args[1], &target) || !GetInt32(env, args[2], &observer) || !GetInt32(env, args[3], &frame)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvUtcTime utc{};
    if (!ReadUtcTime(env, args[4], &utc)) {
        return MakeStatusResult(env, STATUS_INVALID_INPUT);
    }

    DhruvSphericalState st{};
    int32_t status = dhruv_query_utc(
        static_cast<const DhruvEngineHandle*>(ptr),
        target,
        observer,
        frame,
        &utc,
        &st);

    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "state", WriteSphericalState(env, st));
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
    size_t argc = 4;
    napi_value args[4];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 4) return MakeStatusResult(env, STATUS_INVALID_INPUT);

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
napi_value GrahaEnglishName(napi_env env, napi_callback_info info) { return NameLookup(env, info, dhruv_graha_english_name); }
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
    size_t argc = 2;
    napi_value args[2];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 2) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    void* ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvUtcTime utc{};
    if (!ReadUtcTime(env, args[1], &utc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvSankrantiConfig cfg = dhruv_sankranti_config_default();
    DhruvYogaInfo out_yoga{};
    int32_t status = dhruv_yoga_for_date(static_cast<const DhruvEngineHandle*>(ptr), &utc, &cfg, &out_yoga);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "yoga", WriteYogaInfo(env, out_yoga));
    }
    return out;
}

napi_value NakshatraForDate(napi_env env, napi_callback_info info) {
    size_t argc = 2;
    napi_value args[2];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 2) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    void* ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvUtcTime utc{};
    if (!ReadUtcTime(env, args[1], &utc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvSankrantiConfig cfg = dhruv_sankranti_config_default();
    DhruvPanchangNakshatraInfo out_nak{};
    int32_t status = dhruv_nakshatra_for_date(static_cast<const DhruvEngineHandle*>(ptr), &utc, &cfg, &out_nak);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "nakshatra", WritePanchangNakshatraInfo(env, out_nak));
    }
    return out;
}

napi_value VaarForDate(napi_env env, napi_callback_info info) {
    size_t argc = 4;
    napi_value args[4];
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
    size_t argc = 4;
    napi_value args[4];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 4) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvUtcTime utc{};
    DhruvGeoLocation loc{};
    if (!ReadUtcTime(env, args[2], &utc) || !ReadGeoLocation(env, args[3], &loc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvRiseSetConfig rise_cfg = dhruv_riseset_config_default();
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
    size_t argc = 4;
    napi_value args[4];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 4) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    void* e_ptr = nullptr;
    void* ep_ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &e_ptr) || !ReadExternalPtr(env, args[1], &ep_ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvUtcTime utc{};
    DhruvGeoLocation loc{};
    if (!ReadUtcTime(env, args[2], &utc) || !ReadGeoLocation(env, args[3], &loc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvRiseSetConfig rise_cfg = dhruv_riseset_config_default();
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
    size_t argc = 2;
    napi_value args[2];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 2) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    void* ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvUtcTime utc{};
    if (!ReadUtcTime(env, args[1], &utc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvSankrantiConfig cfg = dhruv_sankranti_config_default();
    DhruvMasaInfo out_masa{};
    int32_t status = dhruv_masa_for_date(static_cast<const DhruvEngineHandle*>(ptr), &utc, &cfg, &out_masa);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "masa", WriteMasaInfo(env, out_masa));
    }
    return out;
}

napi_value AyanaForDate(napi_env env, napi_callback_info info) {
    size_t argc = 2;
    napi_value args[2];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 2) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    void* ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvUtcTime utc{};
    if (!ReadUtcTime(env, args[1], &utc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvSankrantiConfig cfg = dhruv_sankranti_config_default();
    DhruvAyanaInfo out_ayana{};
    int32_t status = dhruv_ayana_for_date(static_cast<const DhruvEngineHandle*>(ptr), &utc, &cfg, &out_ayana);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "ayana", WriteAyanaInfo(env, out_ayana));
    }
    return out;
}

napi_value VarshaForDate(napi_env env, napi_callback_info info) {
    size_t argc = 2;
    napi_value args[2];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    if (argc < 2) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    void* ptr = nullptr;
    if (!ReadExternalPtr(env, args[0], &ptr)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvUtcTime utc{};
    if (!ReadUtcTime(env, args[1], &utc)) return MakeStatusResult(env, STATUS_INVALID_INPUT);

    DhruvSankrantiConfig cfg = dhruv_sankranti_config_default();
    DhruvVarshaInfo out_varsha{};
    int32_t status = dhruv_varsha_for_date(static_cast<const DhruvEngineHandle*>(ptr), &utc, &cfg, &out_varsha);
    napi_value out = MakeStatusResult(env, status);
    if (status == STATUS_OK) {
        SetNamed(env, out, "varsha", WriteVarshaInfo(env, out_varsha));
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

napi_value ShadbalaForDate(napi_env env, napi_callback_info info) {
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
    if (status == STATUS_OK) {
        napi_value arr;
        napi_create_array_with_length(env, 7, &arr);
        for (uint32_t i = 0; i < 7; ++i) {
            napi_set_element(env, arr, i, MakeDouble(env, out_result.entries[i].total_rupas));
        }
        SetNamed(env, out, "totalRupas", arr);
    }

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
        SetNamed(env, out, "panchangValid", MakeBool(env, out_result.panchang_valid != 0));
        SetNamed(env, out, "dashaSnapshotCount", MakeUint32(env, out_result.dasha_snapshot_count));
        dhruv_full_kundali_result_free(&out_result);
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
            const DhruvDashaPeriod& p = snapshot.periods[i];
            napi_value po;
            napi_create_object(env, &po);
            SetNamed(env, po, "entityType", MakeUint32(env, p.entity_type));
            SetNamed(env, po, "entityIndex", MakeUint32(env, p.entity_index));
            SetNamed(env, po, "startJd", MakeDouble(env, p.start_jd));
            SetNamed(env, po, "endJd", MakeDouble(env, p.end_jd));
            SetNamed(env, po, "level", MakeUint32(env, p.level));
            SetNamed(env, po, "order", MakeUint32(env, p.order));
            SetNamed(env, po, "parentIdx", MakeUint32(env, p.parent_idx));
            napi_set_element(env, periods, i, po);
        }
        SetNamed(env, snap, "periods", periods);
        SetNamed(env, out, "snapshot", snap);
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
        {"queryUtcSpherical", nullptr, QueryUtcSpherical, nullptr, nullptr, nullptr, napi_default, nullptr},

        {"lskLoad", nullptr, LskLoad, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"lskFree", nullptr, LskFree, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"eopLoad", nullptr, EopLoad, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"eopFree", nullptr, EopFree, nullptr, nullptr, nullptr, napi_default, nullptr},

        {"utcToTdbJd", nullptr, UtcToTdbJd, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"jdTdbToUtc", nullptr, JdTdbToUtc, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"nutationIau2000b", nullptr, NutationIau2000b, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"ayanamshaSystemCount", nullptr, AyanamshaSystemCount, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"referencePlaneDefault", nullptr, ReferencePlaneDefault, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"ayanamshaComputeEx", nullptr, AyanamshaComputeEx, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"lunarNodeCount", nullptr, LunarNodeCount, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"lunarNodeDeg", nullptr, LunarNodeDeg, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"lunarNodeDegWithEngine", nullptr, LunarNodeDegWithEngine, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"rashiCount", nullptr, RashiCount, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"nakshatraCount", nullptr, NakshatraCount, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"rashiFromLongitude", nullptr, RashiFromLongitude, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"nakshatraFromLongitude", nullptr, NakshatraFromLongitude, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"nakshatra28FromLongitude", nullptr, Nakshatra28FromLongitude, nullptr, nullptr, nullptr, napi_default, nullptr},
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
        {"grahaEnglishName", nullptr, GrahaEnglishName, nullptr, nullptr, nullptr, napi_default, nullptr},
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
        {"sankrantiConfigDefault", nullptr, SankrantiConfigDefault, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"conjunctionConfigDefault", nullptr, ConjunctionConfigDefault, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"grahanConfigDefault", nullptr, GrahanConfigDefault, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"stationaryConfigDefault", nullptr, StationaryConfigDefault, nullptr, nullptr, nullptr, napi_default, nullptr},

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
        {"grahaSiderealLongitudes", nullptr, GrahaSiderealLongitudes, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"shadbalaForDate", nullptr, ShadbalaForDate, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"fullKundaliSummaryForDate", nullptr, FullKundaliSummaryForDate, nullptr, nullptr, nullptr, napi_default, nullptr},

        {"dashaSnapshotUtc", nullptr, DashaSnapshotUtc, nullptr, nullptr, nullptr, napi_default, nullptr},

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
