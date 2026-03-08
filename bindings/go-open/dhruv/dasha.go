package dhruv

import (
	"runtime"

	"ctara-dhruv-core/bindings/go-open/internal/cabi"
)

type DashaHierarchy struct {
	h cabi.DashaHierarchyHandle
}

func DashaVariationConfigDefault() DashaVariationConfig { return cabi.DashaVariationConfigDefault() }

func (e *Engine) DashaHierarchyUTC(ep *EOP, birthUTC UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, riseCfg RiseSetConfig, ayanamshaSystem uint32, useNutation bool, system uint8, maxLevel uint8) (*DashaHierarchy, error) {
	h, st := cabi.DashaHierarchyUTC(e.h, ep.h, birthUTC, loc, bhavaCfg, riseCfg, ayanamshaSystem, useNutation, system, maxLevel)
	if err := statusErr("dasha_hierarchy_utc", st); err != nil {
		return nil, err
	}
	d := &DashaHierarchy{h: h}
	runtime.SetFinalizer(d, (*DashaHierarchy).Close)
	return d, nil
}

func (d *DashaHierarchy) Close() {
	if d == nil {
		return
	}
	runtime.SetFinalizer(d, nil)
	d.h.Free()
}

func (d *DashaHierarchy) LevelCount() (uint8, error) {
	out, st := d.h.LevelCount()
	return out, statusErr("dasha_hierarchy_level_count", st)
}

func (d *DashaHierarchy) PeriodCount(level uint8) (uint32, error) {
	out, st := d.h.PeriodCount(level)
	return out, statusErr("dasha_hierarchy_period_count", st)
}

func (d *DashaHierarchy) PeriodAt(level uint8, idx uint32) (DashaPeriod, error) {
	out, st := d.h.PeriodAt(level, idx)
	return out, statusErr("dasha_hierarchy_period_at", st)
}

func (e *Engine) DashaSnapshotUTC(ep *EOP, birthUTC, queryUTC UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, riseCfg RiseSetConfig, ayanamshaSystem uint32, useNutation bool, system uint8, maxLevel uint8) (DashaSnapshot, error) {
	out, st := cabi.DashaSnapshotUTC(e.h, ep.h, birthUTC, queryUTC, loc, bhavaCfg, riseCfg, ayanamshaSystem, useNutation, system, maxLevel)
	return out, statusErr("dasha_snapshot_utc", st)
}

func (e *Engine) DashaLevel0UTC(ep *EOP, birthUTC UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, riseCfg RiseSetConfig, ayanamshaSystem uint32, useNutation bool, system uint8) ([]DashaPeriod, error) {
	out, st := cabi.DashaLevel0UTC(e.h, ep.h, birthUTC, loc, bhavaCfg, riseCfg, ayanamshaSystem, useNutation, system)
	return out, statusErr("dasha_level0_utc", st)
}

func (e *Engine) DashaLevel0EntityUTC(ep *EOP, birthUTC UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, riseCfg RiseSetConfig, ayanamshaSystem uint32, useNutation bool, system, entityType, entityIndex uint8) (DashaPeriod, bool, error) {
	out, found, st := cabi.DashaLevel0EntityUTC(e.h, ep.h, birthUTC, loc, bhavaCfg, riseCfg, ayanamshaSystem, useNutation, system, entityType, entityIndex)
	return out, found, statusErr("dasha_level0_entity_utc", st)
}

func (e *Engine) DashaChildrenUTC(ep *EOP, birthUTC UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, riseCfg RiseSetConfig, ayanamshaSystem uint32, useNutation bool, system uint8, variationCfg DashaVariationConfig, parent DashaPeriod) ([]DashaPeriod, error) {
	out, st := cabi.DashaChildrenUTC(e.h, ep.h, birthUTC, loc, bhavaCfg, riseCfg, ayanamshaSystem, useNutation, system, variationCfg, parent)
	return out, statusErr("dasha_children_utc", st)
}

func (e *Engine) DashaChildPeriodUTC(ep *EOP, birthUTC UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, riseCfg RiseSetConfig, ayanamshaSystem uint32, useNutation bool, system uint8, variationCfg DashaVariationConfig, parent DashaPeriod, childEntityType, childEntityIndex uint8) (DashaPeriod, bool, error) {
	out, found, st := cabi.DashaChildPeriodUTC(e.h, ep.h, birthUTC, loc, bhavaCfg, riseCfg, ayanamshaSystem, useNutation, system, variationCfg, parent, childEntityType, childEntityIndex)
	return out, found, statusErr("dasha_child_period_utc", st)
}

func (e *Engine) DashaCompleteLevelUTC(ep *EOP, birthUTC UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, riseCfg RiseSetConfig, ayanamshaSystem uint32, useNutation bool, system uint8, variationCfg DashaVariationConfig, parentPeriods []DashaPeriod, childLevel uint8) ([]DashaPeriod, error) {
	out, st := cabi.DashaCompleteLevelUTC(e.h, ep.h, birthUTC, loc, bhavaCfg, riseCfg, ayanamshaSystem, useNutation, system, variationCfg, parentPeriods, childLevel)
	return out, statusErr("dasha_complete_level_utc", st)
}
