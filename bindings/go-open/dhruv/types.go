package dhruv

import "ctara-dhruv-core/bindings/go-open/internal/cabi"

const ExpectedAPIVersion = 57

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

const (
	UpagrahaPointStart  = cabi.UpagrahaPointStart
	UpagrahaPointMiddle = cabi.UpagrahaPointMiddle
	UpagrahaPointEnd    = cabi.UpagrahaPointEnd
)

const (
	NaisargikaFriend  = cabi.NaisargikaFriend
	NaisargikaEnemy   = cabi.NaisargikaEnemy
	NaisargikaNeutral = cabi.NaisargikaNeutral
)

const (
	TatkalikaFriend = cabi.TatkalikaFriend
	TatkalikaEnemy  = cabi.TatkalikaEnemy
)

const (
	PanchadhaAdhiShatru = cabi.PanchadhaAdhiShatru
	PanchadhaShatru     = cabi.PanchadhaShatru
	PanchadhaSama       = cabi.PanchadhaSama
	PanchadhaMitra      = cabi.PanchadhaMitra
	PanchadhaAdhiMitra  = cabi.PanchadhaAdhiMitra
)

const (
	DignityExalted      = cabi.DignityExalted
	DignityMoolatrikone = cabi.DignityMoolatrikone
	DignityOwnSign      = cabi.DignityOwnSign
	DignityAdhiMitra    = cabi.DignityAdhiMitra
	DignityMitra        = cabi.DignityMitra
	DignitySama         = cabi.DignitySama
	DignityShatru       = cabi.DignityShatru
	DignityAdhiShatru   = cabi.DignityAdhiShatru
	DignityDebilitated  = cabi.DignityDebilitated
)

const (
	NodeDignitySignLordBased = cabi.NodeDignitySignLordBased
	NodeDignityAlwaysSama    = cabi.NodeDignityAlwaysSama
)

const (
	BeneficNatureBenefic = cabi.BeneficNatureBenefic
	BeneficNatureMalefic = cabi.BeneficNatureMalefic
)

const (
	GrahaGenderMale   = cabi.GrahaGenderMale
	GrahaGenderFemale = cabi.GrahaGenderFemale
	GrahaGenderNeuter = cabi.GrahaGenderNeuter
)

const (
	GulikaMaandiPlanetRahu   = cabi.GulikaMaandiPlanetRahu
	GulikaMaandiPlanetSaturn = cabi.GulikaMaandiPlanetSaturn
)

const (
	QueryTimeJDTDB = cabi.QueryTimeJDTDB
	QueryTimeUTC   = cabi.QueryTimeUTC
)

const (
	SearchTimeJDTDB = cabi.SearchTimeJDTDB
	SearchTimeUTC   = cabi.SearchTimeUTC
)

const (
	DefaultsModeRecommended = cabi.DefaultsModeRecommended
	DefaultsModeNone        = cabi.DefaultsModeNone
)

const (
	DashaTimeNone  = cabi.DashaTimeNone
	DashaTimeJDUTC = cabi.DashaTimeJDUTC
	DashaTimeUTC   = cabi.DashaTimeUTC
)

const (
	QueryOutputCartesian = cabi.QueryOutputCartesian
	QueryOutputSpherical = cabi.QueryOutputSpherical
	QueryOutputBoth      = cabi.QueryOutputBoth
)

const (
	PrecessionModelNewcomb1895 = cabi.PrecessionModelNewcomb1895
	PrecessionModelLieske1977  = cabi.PrecessionModelLieske1977
	PrecessionModelIau2006     = cabi.PrecessionModelIau2006
	PrecessionModelVondrak2011 = cabi.PrecessionModelVondrak2011
)

const (
	GrahaLongitudeKindSidereal = cabi.GrahaLongitudeKindSidereal
	GrahaLongitudeKindTropical = cabi.GrahaLongitudeKindTropical
)

const (
	TimePolicyStrictLSK    = cabi.TimePolicyStrictLSK
	TimePolicyHybridDeltaT = cabi.TimePolicyHybridDeltaT
)

const (
	DeltaTModelLegacyEspenakMeeus2006     = cabi.DeltaTModelLegacyEspenakMeeus2006
	DeltaTModelSmh2016WithPre720Quadratic = cabi.DeltaTModelSmh2016WithPre720Quadratic
)

const (
	FutureDeltaTTransitionLegacyTtUtcBlend         = cabi.FutureDeltaTTransitionLegacyTtUtcBlend
	FutureDeltaTTransitionBridgeFromModernEndpoint = cabi.FutureDeltaTTransitionBridgeFromModernEndpoint
)

const (
	SmhFutureFamilyAddendum2020Piecewise = cabi.SmhFutureFamilyAddendum2020Piecewise
	SmhFutureFamilyConstantCMinus20      = cabi.SmhFutureFamilyConstantCMinus20
	SmhFutureFamilyConstantCMinus17p52   = cabi.SmhFutureFamilyConstantCMinus17p52
	SmhFutureFamilyConstantCMinus15p32   = cabi.SmhFutureFamilyConstantCMinus15p32
	SmhFutureFamilyStephenson1997        = cabi.SmhFutureFamilyStephenson1997
	SmhFutureFamilyStephenson2016        = cabi.SmhFutureFamilyStephenson2016
)

const (
	TtUtcSourceLskDeltaAt  = cabi.TtUtcSourceLskDeltaAt
	TtUtcSourceDeltaTModel = cabi.TtUtcSourceDeltaTModel
)

const (
	TimeWarningLskFutureFrozen     = cabi.TimeWarningLskFutureFrozen
	TimeWarningLskPreRangeFallback = cabi.TimeWarningLskPreRangeFallback
	TimeWarningEopFutureFrozen     = cabi.TimeWarningEopFutureFrozen
	TimeWarningEopPreRangeFallback = cabi.TimeWarningEopPreRangeFallback
	TimeWarningDeltaTModelUsed     = cabi.TimeWarningDeltaTModelUsed
	MaxTimeWarnings                = cabi.MaxTimeWarnings
)

type (
	EngineConfig            = cabi.EngineConfig
	ConfigLoadOptions       = cabi.ConfigLoadOptions
	Query                   = cabi.Query
	QueryRequest            = cabi.QueryRequest
	QueryResult             = cabi.QueryResult
	StateVector             = cabi.StateVector
	SphericalCoords         = cabi.SphericalCoords
	SphericalState          = cabi.SphericalState
	UtcTime                 = cabi.UtcTime
	TimeConversionOptions   = cabi.TimeConversionOptions
	TimePolicy              = cabi.TimePolicy
	TimeWarning             = cabi.TimeWarning
	TimeDiagnostics         = cabi.TimeDiagnostics
	UtcToTdbRequest         = cabi.UtcToTdbRequest
	UtcToTdbResult          = cabi.UtcToTdbResult
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

	GrahaLongitudes           = cabi.GrahaLongitudes
	GrahaLongitudesConfig     = cabi.GrahaLongitudesConfig
	SpecialLagnas             = cabi.SpecialLagnas
	ArudhaResult              = cabi.ArudhaResult
	AllUpagrahas              = cabi.AllUpagrahas
	TimeUpagrahaConfig        = cabi.TimeUpagrahaConfig
	DrishtiConfig             = cabi.DrishtiConfig
	GrahaPositionsConfig      = cabi.GrahaPositionsConfig
	BindusConfig              = cabi.BindusConfig
	DashaSnapshotTime         = cabi.DashaSnapshotTime
	DashaSelectionConfig      = cabi.DashaSelectionConfig
	DashaVariationConfig      = cabi.DashaVariationConfig
	RashiDashaInputs          = cabi.RashiDashaInputs
	DashaInputs               = cabi.DashaInputs
	DashaBirthContext         = cabi.DashaBirthContext
	DashaHierarchyRequest     = cabi.DashaHierarchyRequest
	DashaSnapshotRequest      = cabi.DashaSnapshotRequest
	DashaLevel0Request        = cabi.DashaLevel0Request
	DashaLevel0EntityRequest  = cabi.DashaLevel0EntityRequest
	DashaChildrenRequest      = cabi.DashaChildrenRequest
	DashaChildPeriodRequest   = cabi.DashaChildPeriodRequest
	DashaCompleteLevelRequest = cabi.DashaCompleteLevelRequest
	AmshaSelectionConfig      = cabi.AmshaSelectionConfig
	FullKundaliConfig         = cabi.FullKundaliConfig

	TaraConfig         = cabi.TaraConfig
	EarthState         = cabi.EarthState
	EquatorialPosition = cabi.EquatorialPosition
	TaraComputeRequest = cabi.TaraComputeRequest
	TaraComputeResult  = cabi.TaraComputeResult

	DashaPeriod   = cabi.DashaPeriod
	DashaSnapshot = cabi.DashaSnapshot

	SthanaBalaBreakdown       = cabi.SthanaBalaBreakdown
	KalaBalaBreakdown         = cabi.KalaBalaBreakdown
	ShadbalaEntry             = cabi.ShadbalaEntry
	ShadbalaResult            = cabi.ShadbalaResult
	BhavaBalaEntry            = cabi.BhavaBalaEntry
	BhavaBalaResult           = cabi.BhavaBalaResult
	BhavaBalaInputs           = cabi.BhavaBalaInputs
	VimsopakaEntry            = cabi.VimsopakaEntry
	VimsopakaResult           = cabi.VimsopakaResult
	BalaBundleResult          = cabi.BalaBundleResult
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

	AmshaEntry            = cabi.AmshaEntry
	AmshaChartScope       = cabi.AmshaChartScope
	AmshaChart            = cabi.AmshaChart
	AmshaVariationInfo    = cabi.AmshaVariationInfo
	AmshaVariationCatalog = cabi.AmshaVariationCatalog
)
