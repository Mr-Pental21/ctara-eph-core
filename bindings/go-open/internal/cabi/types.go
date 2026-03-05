package cabi

const (
	PathCapacity          = 512
	MaxSpkPaths           = 8
	GrahaCount            = 9
	SaptaGrahaCount       = 7
	SphutaCount           = 16
	UpagrahaCount         = 11
	MaxCharakarakaEntries = 8
)

const (
	CharakarakaSchemeEight           uint8 = 0
	CharakarakaSchemeSevenNoPitri    uint8 = 1
	CharakarakaSchemeSevenPkMergedMk uint8 = 2
	CharakarakaSchemeMixedParashara  uint8 = 3
)

const (
	CharakarakaRoleAtma       uint8 = 0
	CharakarakaRoleAmatya     uint8 = 1
	CharakarakaRoleBhratri    uint8 = 2
	CharakarakaRoleMatri      uint8 = 3
	CharakarakaRolePitri      uint8 = 4
	CharakarakaRolePutra      uint8 = 5
	CharakarakaRoleGnati      uint8 = 6
	CharakarakaRoleDara       uint8 = 7
	CharakarakaRoleMatriPutra uint8 = 8
)

type Status int32

const (
	StatusOK                  Status = 0
	StatusInvalidConfig       Status = 1
	StatusInvalidQuery        Status = 2
	StatusKernelLoad          Status = 3
	StatusTimeConversion      Status = 4
	StatusUnsupportedQuery    Status = 5
	StatusEpochOutOfRange     Status = 6
	StatusNullPointer         Status = 7
	StatusEopLoad             Status = 8
	StatusEopOutOfRange       Status = 9
	StatusInvalidLocation     Status = 10
	StatusNoConvergence       Status = 11
	StatusInvalidSearchConfig Status = 12
	StatusInvalidInput        Status = 13
	StatusInternal            Status = 255
)

type EngineConfig struct {
	SpkPaths         []string
	LskPath          string
	CacheCapacity    uint64
	StrictValidation bool
}

type Query struct {
	Target     int32
	Observer   int32
	Frame      int32
	EpochTdbJD float64
}

type StateVector struct {
	PositionKm [3]float64
	VelocityKm [3]float64
}

type SphericalCoords struct {
	LonDeg     float64
	LatDeg     float64
	DistanceKm float64
}

type SphericalState struct {
	LonDeg        float64
	LatDeg        float64
	DistanceKm    float64
	LonSpeed      float64
	LatSpeed      float64
	DistanceSpeed float64
}

type UtcTime struct {
	Year   int32
	Month  uint32
	Day    uint32
	Hour   uint32
	Minute uint32
	Second float64
}

type GeoLocation struct {
	LatitudeDeg  float64
	LongitudeDeg float64
	AltitudeM    float64
}

type RiseSetConfig struct {
	UseRefraction      bool
	SunLimb            int32
	AltitudeCorrection bool
}

type RiseSetResult struct {
	ResultType int32
	EventCode  int32
	JdTdb      float64
}

type RiseSetResultUTC struct {
	ResultType int32
	EventCode  int32
	UTC        UtcTime
}

type AyanamshaComputeRequest struct {
	SystemCode     int32
	Mode           int32
	TimeKind       int32
	JdTdb          float64
	UTC            UtcTime
	UseNutation    bool
	DeltaPsiArcsec float64
}

type BhavaConfig struct {
	System         int32
	StartingPoint  int32
	CustomStartDeg float64
	ReferenceMode  int32
}

type Bhava struct {
	Number   uint8
	CuspDeg  float64
	StartDeg float64
	EndDeg   float64
}

type BhavaResult struct {
	Bhavas   [12]Bhava
	LagnaDeg float64
	MCDeg    float64
}

type LunarNodeRequest struct {
	NodeCode int32
	ModeCode int32
	Backend  int32
	TimeKind int32
	JdTdb    float64
	UTC      UtcTime
}

type ConjunctionConfig struct {
	TargetSeparationDeg float64
	StepSizeDays        float64
	MaxIterations       uint32
	ConvergenceDays     float64
}

type ConjunctionSearchRequest struct {
	Body1Code  int32
	Body2Code  int32
	QueryMode  int32
	AtJdTdb    float64
	StartJdTdb float64
	EndJdTdb   float64
	Config     ConjunctionConfig
}

type ConjunctionEvent struct {
	JdTdb               float64
	ActualSeparationDeg float64
	Body1LongitudeDeg   float64
	Body2LongitudeDeg   float64
	Body1LatitudeDeg    float64
	Body2LatitudeDeg    float64
	Body1Code           int32
	Body2Code           int32
}

type GrahanConfig struct {
	IncludePenumbral   bool
	IncludePeakDetails bool
}

type GrahanSearchRequest struct {
	GrahanKind int32
	QueryMode  int32
	AtJdTdb    float64
	StartJdTdb float64
	EndJdTdb   float64
	Config     GrahanConfig
}

type ChandraGrahanResult struct {
	GrahanType           int32
	Magnitude            float64
	PenumbralMagnitude   float64
	GreatestGrahanJd     float64
	P1Jd                 float64
	U1Jd                 float64
	U2Jd                 float64
	U3Jd                 float64
	U4Jd                 float64
	P4Jd                 float64
	MoonEclipticLatDeg   float64
	AngularSeparationDeg float64
}

type SuryaGrahanResult struct {
	GrahanType           int32
	Magnitude            float64
	GreatestGrahanJd     float64
	C1Jd                 float64
	C2Jd                 float64
	C3Jd                 float64
	C4Jd                 float64
	MoonEclipticLatDeg   float64
	AngularSeparationDeg float64
}

type StationaryConfig struct {
	StepSizeDays      float64
	MaxIterations     uint32
	ConvergenceDays   float64
	NumericalStepDays float64
}

type MotionSearchRequest struct {
	BodyCode   int32
	MotionKind int32
	QueryMode  int32
	AtJdTdb    float64
	StartJdTdb float64
	EndJdTdb   float64
	Config     StationaryConfig
}

type StationaryEvent struct {
	JdTdb        float64
	BodyCode     int32
	LongitudeDeg float64
	LatitudeDeg  float64
	StationType  int32
}

type MaxSpeedEvent struct {
	JdTdb          float64
	BodyCode       int32
	LongitudeDeg   float64
	LatitudeDeg    float64
	SpeedDegPerDay float64
	SpeedType      int32
}

type SankrantiConfig struct {
	AyanamshaSystem int32
	UseNutation     bool
	ReferencePlane  int32
	StepSizeDays    float64
	MaxIterations   uint32
	ConvergenceDays float64
}

type SankrantiEvent struct {
	UTC                     UtcTime
	RashiIndex              int32
	SunSiderealLongitudeDeg float64
	SunTropicalLongitudeDeg float64
}

type SankrantiSearchRequest struct {
	TargetKind int32
	QueryMode  int32
	RashiIndex int32
	AtJdTdb    float64
	StartJdTdb float64
	EndJdTdb   float64
	Config     SankrantiConfig
}

type LunarPhaseSearchRequest struct {
	PhaseKind  int32
	QueryMode  int32
	AtJdTdb    float64
	StartJdTdb float64
	EndJdTdb   float64
}

type LunarPhaseEvent struct {
	UTC              UtcTime
	Phase            int32
	MoonLongitudeDeg float64
	SunLongitudeDeg  float64
}

type TithiInfo struct {
	TithiIndex    int32
	Paksha        int32
	TithiInPaksha int32
	Start         UtcTime
	End           UtcTime
}

type KaranaInfo struct {
	KaranaIndex     int32
	KaranaNameIndex int32
	Start           UtcTime
	End             UtcTime
}

type YogaInfo struct {
	YogaIndex int32
	Start     UtcTime
	End       UtcTime
}

type VaarInfo struct {
	VaarIndex int32
	Start     UtcTime
	End       UtcTime
}

type HoraInfo struct {
	HoraIndex    int32
	HoraPosition int32
	Start        UtcTime
	End          UtcTime
}

type GhatikaInfo struct {
	Value int32
	Start UtcTime
	End   UtcTime
}

type PanchangNakshatraInfo struct {
	NakshatraIndex int32
	Pada           int32
	Start          UtcTime
	End            UtcTime
}

type MasaInfo struct {
	MasaIndex int32
	Adhika    bool
	Start     UtcTime
	End       UtcTime
}

type AyanaInfo struct {
	Ayana int32
	Start UtcTime
	End   UtcTime
}

type VarshaInfo struct {
	SamvatsaraIndex int32
	Order           int32
	Start           UtcTime
	End             UtcTime
}

type PanchangComputeRequest struct {
	TimeKind        int32
	JdTdb           float64
	UTC             UtcTime
	IncludeMask     uint32
	Location        GeoLocation
	RiseSetConfig   RiseSetConfig
	SankrantiConfig SankrantiConfig
}

type PanchangOperationResult struct {
	TithiValid     bool
	Tithi          TithiInfo
	KaranaValid    bool
	Karana         KaranaInfo
	YogaValid      bool
	Yoga           YogaInfo
	VaarValid      bool
	Vaar           VaarInfo
	HoraValid      bool
	Hora           HoraInfo
	GhatikaValid   bool
	Ghatika        GhatikaInfo
	NakshatraValid bool
	Nakshatra      PanchangNakshatraInfo
	MasaValid      bool
	Masa           MasaInfo
	AyanaValid     bool
	Ayana          AyanaInfo
	VarshaValid    bool
	Varsha         VarshaInfo
}

type GrahaLongitudes struct {
	Longitudes [GrahaCount]float64
}

type SpecialLagnas struct {
	BhavaLagna     float64
	HoraLagna      float64
	GhatiLagna     float64
	VighatiLagna   float64
	VarnadaLagna   float64
	SreeLagna      float64
	PranapadaLagna float64
	InduLagna      float64
}

type ArudhaResult struct {
	BhavaNumber  uint8
	LongitudeDeg float64
	RashiIndex   uint8
}

type AllUpagrahas struct {
	Gulika       float64
	Maandi       float64
	Kaala        float64
	Mrityu       float64
	ArthaPrahara float64
	YamaGhantaka float64
	Dhooma       float64
	Vyatipata    float64
	Parivesha    float64
	IndraChapa   float64
	Upaketu      float64
}

type DrishtiConfig struct {
	IncludeBhava  bool
	IncludeLagna  bool
	IncludeBindus bool
}

type GrahaPositionsConfig struct {
	IncludeNakshatra    bool
	IncludeLagna        bool
	IncludeOuterPlanets bool
	IncludeBhava        bool
}

type BindusConfig struct {
	IncludeNakshatra bool
	IncludeBhava     bool
}

type TaraConfig struct {
	Accuracy      int32
	ApplyParallax bool
}

type EarthState struct {
	PositionAUDay [3]float64
	VelocityAUDay [3]float64
}

type TaraComputeRequest struct {
	TaraID          int32
	OutputKind      int32
	JdTdb           float64
	AyanamshaDeg    float64
	Config          TaraConfig
	EarthStateValid bool
	EarthState      EarthState
}

type EquatorialPosition struct {
	RADeg      float64
	DecDeg     float64
	DistanceAU float64
}

type TaraComputeResult struct {
	OutputKind           int32
	Equatorial           EquatorialPosition
	Ecliptic             SphericalCoords
	SiderealLongitudeDeg float64
}

type DashaPeriod struct {
	EntityType  uint8
	EntityIndex uint8
	StartJD     float64
	EndJD       float64
	Level       uint8
	Order       uint16
	ParentIdx   uint32
}

type DashaSnapshot struct {
	System  uint8
	QueryJD float64
	Count   uint8
	Periods [5]DashaPeriod
}

type Dms struct {
	Degrees uint16
	Minutes uint8
	Seconds float64
}

type RashiInfo struct {
	RashiIndex     uint8
	Dms            Dms
	DegreesInRashi float64
}

type NakshatraInfo struct {
	NakshatraIndex     uint8
	Pada               uint8
	DegreesInNakshatra float64
	DegreesInPada      float64
}

type Nakshatra28Info struct {
	NakshatraIndex     uint8
	Pada               uint8
	DegreesInNakshatra float64
}

type TithiPosition struct {
	TithiIndex     int32
	Paksha         int32
	TithiInPaksha  int32
	DegreesInTithi float64
}

type KaranaPosition struct {
	KaranaIndex     int32
	DegreesInKarana float64
}

type YogaPosition struct {
	YogaIndex     int32
	DegreesInYoga float64
}

type SamvatsaraResult struct {
	SamvatsaraIndex int32
	CyclePosition   int32
}

type SphutalInputs struct {
	Sun        float64
	Moon       float64
	Mars       float64
	Jupiter    float64
	Venus      float64
	Rahu       float64
	Lagna      float64
	EighthLord float64
	Gulika     float64
}

type SphutalResult struct {
	Longitudes [SphutaCount]float64
}

type DrishtiEntry struct {
	AngularDistance float64
	BaseVirupa      float64
	SpecialVirupa   float64
	TotalVirupa     float64
}

type GrahaDrishtiMatrix struct {
	Entries [GrahaCount][GrahaCount]DrishtiEntry
}

type DrishtiResult struct {
	GrahaToGraha  [GrahaCount][GrahaCount]DrishtiEntry
	GrahaToBhava  [GrahaCount][12]DrishtiEntry
	GrahaToLagna  [GrahaCount]DrishtiEntry
	GrahaToBindus [GrahaCount][19]DrishtiEntry
}

type GrahaEntry struct {
	SiderealLongitude float64
	RashiIndex        uint8
	NakshatraIndex    uint8
	Pada              uint8
	BhavaNumber       uint8
}

type GrahaPositions struct {
	Grahas       [GrahaCount]GrahaEntry
	Lagna        GrahaEntry
	OuterPlanets [3]GrahaEntry
}

type BindusResult struct {
	ArudhaPadas    [12]GrahaEntry
	BhriguBindu    GrahaEntry
	PranapadaLagna GrahaEntry
	Gulika         GrahaEntry
	Maandi         GrahaEntry
	HoraLagna      GrahaEntry
	GhatiLagna     GrahaEntry
	SreeLagna      GrahaEntry
}

type BhinnaAshtakavarga struct {
	GrahaIndex uint8
	Points     [12]uint8
}

type SarvaAshtakavarga struct {
	TotalPoints      [12]uint8
	AfterTrikona     [12]uint8
	AfterEkadhipatya [12]uint8
}

type AshtakavargaResult struct {
	BAVs [SaptaGrahaCount]BhinnaAshtakavarga
	SAV  SarvaAshtakavarga
}

type AmshaEntry struct {
	SiderealLongitude float64
	RashiIndex        uint8
	DmsDegrees        uint16
	DmsMinutes        uint8
	DmsSeconds        float64
	DegreesInRashi    float64
}

type AmshaChartScope struct {
	IncludeBhavaCusps    bool
	IncludeArudhaPadas   bool
	IncludeUpagrahas     bool
	IncludeSphutas       bool
	IncludeSpecialLagnas bool
}

type AmshaChart struct {
	AmshaCode          uint16
	VariationCode      uint8
	Grahas             [GrahaCount]AmshaEntry
	Lagna              AmshaEntry
	BhavaCuspsValid    bool
	ArudhaPadasValid   bool
	UpagrahasValid     bool
	SphutasValid       bool
	SpecialLagnasValid bool
}

type SthanaBalaBreakdown struct {
	Uchcha       float64
	Saptavargaja float64
	Ojhayugma    float64
	Kendradi     float64
	Drekkana     float64
	Total        float64
}

type KalaBalaBreakdown struct {
	Nathonnatha float64
	Paksha      float64
	Tribhaga    float64
	Abda        float64
	Masa        float64
	Vara        float64
	Hora        float64
	Ayana       float64
	Yuddha      float64
	Total       float64
}

type ShadbalaEntry struct {
	GrahaIndex        uint8
	Sthana            SthanaBalaBreakdown
	Dig               float64
	Kala              KalaBalaBreakdown
	Cheshta           float64
	Naisargika        float64
	Drik              float64
	TotalShashtiamsas float64
	TotalRupas        float64
	RequiredStrength  float64
	IsStrong          bool
}

type ShadbalaResult struct {
	Entries [SaptaGrahaCount]ShadbalaEntry
}

type VimsopakaEntry struct {
	GrahaIndex   uint8
	Shadvarga    float64
	Saptavarga   float64
	Dashavarga   float64
	Shodasavarga float64
}

type VimsopakaResult struct {
	Entries [GrahaCount]VimsopakaEntry
}

type SayanadiResult struct {
	Avastha   uint8
	SubStates [5]uint8
}

type GrahaAvasthas struct {
	Baladi    uint8
	Jagradadi uint8
	Deeptadi  uint8
	Lajjitadi uint8
	Sayanadi  SayanadiResult
}

type AllGrahaAvasthas struct {
	Entries [GrahaCount]GrahaAvasthas
}

type CharakarakaEntry struct {
	RoleCode                uint8
	GrahaIndex              uint8
	Rank                    uint8
	LongitudeDeg            float64
	DegreesInRashi          float64
	EffectiveDegreesInRashi float64
}

type CharakarakaResult struct {
	Scheme           uint8
	UsedEightKarakas bool
	Count            uint8
	Entries          [MaxCharakarakaEntries]CharakarakaEntry
}

type FullKundaliSummary struct {
	AyanamshaDeg        float64
	BhavaCuspsValid     bool
	GrahaPositionsValid bool
	BindusValid         bool
	DrishtiValid        bool
	AshtakavargaValid   bool
	UpagrahasValid      bool
	SpecialLagnasValid  bool
	AmshasValid         bool
	AmshasCount         uint8
	ShadbalaValid       bool
	VimsopakaValid      bool
	AvasthaValid        bool
	CharakarakaValid    bool
	PanchangValid       bool
	DashaCount          uint8
	DashaSnapshotCount  uint8
}
