package dhruv

import "ctara-dhruv-core/bindings/go-open/internal/cabi"

const ExpectedAPIVersion = 45

const (
	PathCapacity          = cabi.PathCapacity
	MaxSpkPaths           = cabi.MaxSpkPaths
	GrahaCount            = cabi.GrahaCount
	SphutaCount           = cabi.SphutaCount
	MaxDashaSystems       = cabi.MaxDashaSystems
	MaxCharakarakaEntries = cabi.MaxCharakarakaEntries
)

const (
	CharakarakaSchemeEight           = cabi.CharakarakaSchemeEight
	CharakarakaSchemeSevenNoPitri    = cabi.CharakarakaSchemeSevenNoPitri
	CharakarakaSchemeSevenPkMergedMk = cabi.CharakarakaSchemeSevenPkMergedMk
	CharakarakaSchemeMixedParashara  = cabi.CharakarakaSchemeMixedParashara
)

const (
	CharakarakaRoleAtma       = cabi.CharakarakaRoleAtma
	CharakarakaRoleAmatya     = cabi.CharakarakaRoleAmatya
	CharakarakaRoleBhratri    = cabi.CharakarakaRoleBhratri
	CharakarakaRoleMatri      = cabi.CharakarakaRoleMatri
	CharakarakaRolePitri      = cabi.CharakarakaRolePitri
	CharakarakaRolePutra      = cabi.CharakarakaRolePutra
	CharakarakaRoleGnati      = cabi.CharakarakaRoleGnati
	CharakarakaRoleDara       = cabi.CharakarakaRoleDara
	CharakarakaRoleMatriPutra = cabi.CharakarakaRoleMatriPutra
)

type (
	EngineConfig            = cabi.EngineConfig
	Query                   = cabi.Query
	StateVector             = cabi.StateVector
	SphericalCoords         = cabi.SphericalCoords
	SphericalState          = cabi.SphericalState
	UtcTime                 = cabi.UtcTime
	GeoLocation             = cabi.GeoLocation
	RiseSetConfig           = cabi.RiseSetConfig
	RiseSetResult           = cabi.RiseSetResult
	RiseSetResultUTC        = cabi.RiseSetResultUTC
	AyanamshaComputeRequest = cabi.AyanamshaComputeRequest
	BhavaConfig             = cabi.BhavaConfig
	BhavaResult             = cabi.BhavaResult
	LunarNodeRequest        = cabi.LunarNodeRequest

	ConjunctionConfig        = cabi.ConjunctionConfig
	ConjunctionSearchRequest = cabi.ConjunctionSearchRequest
	ConjunctionEvent         = cabi.ConjunctionEvent

	GrahanConfig        = cabi.GrahanConfig
	GrahanSearchRequest = cabi.GrahanSearchRequest
	ChandraGrahanResult = cabi.ChandraGrahanResult
	SuryaGrahanResult   = cabi.SuryaGrahanResult

	StationaryConfig    = cabi.StationaryConfig
	MotionSearchRequest = cabi.MotionSearchRequest
	StationaryEvent     = cabi.StationaryEvent
	MaxSpeedEvent       = cabi.MaxSpeedEvent

	SankrantiConfig         = cabi.SankrantiConfig
	SankrantiSearchRequest  = cabi.SankrantiSearchRequest
	SankrantiEvent          = cabi.SankrantiEvent
	LunarPhaseSearchRequest = cabi.LunarPhaseSearchRequest
	LunarPhaseEvent         = cabi.LunarPhaseEvent

	TithiInfo               = cabi.TithiInfo
	KaranaInfo              = cabi.KaranaInfo
	YogaInfo                = cabi.YogaInfo
	VaarInfo                = cabi.VaarInfo
	HoraInfo                = cabi.HoraInfo
	GhatikaInfo             = cabi.GhatikaInfo
	PanchangNakshatraInfo   = cabi.PanchangNakshatraInfo
	MasaInfo                = cabi.MasaInfo
	AyanaInfo               = cabi.AyanaInfo
	VarshaInfo              = cabi.VarshaInfo
	PanchangComputeRequest  = cabi.PanchangComputeRequest
	PanchangOperationResult = cabi.PanchangOperationResult

	GrahaLongitudes      = cabi.GrahaLongitudes
	SpecialLagnas        = cabi.SpecialLagnas
	ArudhaResult         = cabi.ArudhaResult
	AllUpagrahas         = cabi.AllUpagrahas
	DrishtiConfig        = cabi.DrishtiConfig
	GrahaPositionsConfig = cabi.GrahaPositionsConfig
	BindusConfig         = cabi.BindusConfig
	DashaSelectionConfig = cabi.DashaSelectionConfig
	DashaVariationConfig = cabi.DashaVariationConfig
	AmshaSelectionConfig = cabi.AmshaSelectionConfig
	FullKundaliConfig    = cabi.FullKundaliConfig

	TaraConfig         = cabi.TaraConfig
	EarthState         = cabi.EarthState
	TaraComputeRequest = cabi.TaraComputeRequest
	TaraComputeResult  = cabi.TaraComputeResult

	DashaPeriod   = cabi.DashaPeriod
	DashaSnapshot = cabi.DashaSnapshot

	SthanaBalaBreakdown       = cabi.SthanaBalaBreakdown
	KalaBalaBreakdown         = cabi.KalaBalaBreakdown
	ShadbalaEntry             = cabi.ShadbalaEntry
	ShadbalaResult            = cabi.ShadbalaResult
	VimsopakaEntry            = cabi.VimsopakaEntry
	VimsopakaResult           = cabi.VimsopakaResult
	SayanadiResult            = cabi.SayanadiResult
	GrahaAvasthas             = cabi.GrahaAvasthas
	AllGrahaAvasthas          = cabi.AllGrahaAvasthas
	CharakarakaEntry          = cabi.CharakarakaEntry
	CharakarakaResult         = cabi.CharakarakaResult
	FullKundaliSummary        = cabi.FullKundaliSummary
	FullPanchangInfo          = cabi.FullPanchangInfo
	FullKundaliDashaLevel     = cabi.FullKundaliDashaLevel
	FullKundaliDashaHierarchy = cabi.FullKundaliDashaHierarchy
	FullKundaliResult         = cabi.FullKundaliResult

	Dms             = cabi.Dms
	RashiInfo       = cabi.RashiInfo
	NakshatraInfo   = cabi.NakshatraInfo
	Nakshatra28Info = cabi.Nakshatra28Info

	TithiPosition    = cabi.TithiPosition
	KaranaPosition   = cabi.KaranaPosition
	YogaPosition     = cabi.YogaPosition
	SamvatsaraResult = cabi.SamvatsaraResult

	SphutalInputs = cabi.SphutalInputs
	SphutalResult = cabi.SphutalResult

	DrishtiEntry       = cabi.DrishtiEntry
	GrahaDrishtiMatrix = cabi.GrahaDrishtiMatrix
	DrishtiResult      = cabi.DrishtiResult

	GrahaEntry     = cabi.GrahaEntry
	GrahaPositions = cabi.GrahaPositions
	BindusResult   = cabi.BindusResult

	BhinnaAshtakavarga = cabi.BhinnaAshtakavarga
	SarvaAshtakavarga  = cabi.SarvaAshtakavarga
	AshtakavargaResult = cabi.AshtakavargaResult

	AmshaEntry      = cabi.AmshaEntry
	AmshaChartScope = cabi.AmshaChartScope
	AmshaChart      = cabi.AmshaChart
)
