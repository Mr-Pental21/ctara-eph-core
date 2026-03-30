package dhruv

import (
	"runtime"

	"ctara-dhruv-core/bindings/go-open/internal/cabi"
)

type DashaHierarchy struct {
	h cabi.DashaHierarchyHandle
}

func DashaVariationConfigDefault() DashaVariationConfig { return cabi.DashaVariationConfigDefault() }

func (e *Engine) DashaHierarchy(ep *EOP, request DashaHierarchyRequest) (*DashaHierarchy, error) {
	h, st := cabi.DashaHierarchy(e.h, ep.h, request)
	if err := statusErr("dasha_hierarchy", st); err != nil {
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

func (e *Engine) DashaSnapshot(ep *EOP, request DashaSnapshotRequest) (DashaSnapshot, error) {
	out, st := cabi.RunDashaSnapshot(e.h, ep.h, request)
	return out, statusErr("dasha_snapshot", st)
}

func (e *Engine) DashaLevel0(ep *EOP, request DashaLevel0Request) ([]DashaPeriod, error) {
	out, st := cabi.DashaLevel0(e.h, ep.h, request)
	return out, statusErr("dasha_level0", st)
}

func (e *Engine) DashaLevel0Entity(ep *EOP, request DashaLevel0EntityRequest) (DashaPeriod, bool, error) {
	out, found, st := cabi.DashaLevel0Entity(e.h, ep.h, request)
	return out, found, statusErr("dasha_level0_entity", st)
}

func (e *Engine) DashaChildren(ep *EOP, request DashaChildrenRequest) ([]DashaPeriod, error) {
	out, st := cabi.DashaChildren(e.h, ep.h, request)
	return out, statusErr("dasha_children", st)
}

func (e *Engine) DashaChildPeriod(ep *EOP, request DashaChildPeriodRequest) (DashaPeriod, bool, error) {
	out, found, st := cabi.DashaChildPeriod(e.h, ep.h, request)
	return out, found, statusErr("dasha_child_period", st)
}

func (e *Engine) DashaCompleteLevel(ep *EOP, request DashaCompleteLevelRequest, parentPeriods []DashaPeriod) ([]DashaPeriod, error) {
	out, st := cabi.DashaCompleteLevel(e.h, ep.h, request, parentPeriods)
	return out, statusErr("dasha_complete_level", st)
}
