use serde::{Deserialize, Serialize};

/// Outcome of the code execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Outcome {
    OutcomeUnspecified,
    OutcomeOk,
    OutcomeFailed,
    OutcomeDeadlineExceeded,
}

/// Programming language of the executable code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Language {
    LanguageUnspecified,
    Python,
}

/// Specifies how the response should be scheduled in the conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FunctionResponseScheduling {
    SchedulingUnspecified,
    Silent,
    WhenIdle,
    Interrupt,
}

/// The JSON Schema data type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Type {
    TypeUnspecified,
    String,
    Number,
    Integer,
    Boolean,
    Array,
    Object,
    Null,
}

/// Server content modalities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MediaModality {
    ModalityUnspecified,
    Text,
    Image,
    Video,
    Audio,
    Document,
}

/// The type of the VAD signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VadSignalType {
    VadSignalTypeUnspecified,
    VadSignalTypeSos,
    VadSignalTypeEos,
}

/// Start of speech sensitivity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StartSensitivity {
    StartSensitivityUnspecified,
    StartSensitivityHigh,
    StartSensitivityLow,
}

/// End of speech sensitivity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EndSensitivity {
    EndSensitivityUnspecified,
    EndSensitivityHigh,
    EndSensitivityLow,
}

/// The different ways of handling user activity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ActivityHandling {
    ActivityHandlingUnspecified,
    StartOfActivityInterrupts,
    NoInterruption,
}

/// Options about which input is included in the user's turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TurnCoverage {
    TurnCoverageUnspecified,
    TurnIncludesOnlyActivity,
    TurnIncludesAllInput,
}

/// Response modalities for generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Modality {
    ModalityUnspecified,
    Text,
    Image,
    Audio,
}

/// The media resolution to use.
///
/// 参考 token 映射（不同媒体类型不同）：
/// - 图像: `LOW/MEDIUM/HIGH/ULTRA_HIGH` ≈ 280/560/1120/2240 tokens
/// - 视频: `LOW/MEDIUM/HIGH` ≈ 70/70/280 tokens per frame
/// - PDF: `LOW/MEDIUM/HIGH` ≈ 280/560/1120 tokens + 原生文本（Gemini 3）
///
/// 实际计费与限制以服务端为准。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MediaResolution {
    MediaResolutionUnspecified,
    MediaResolutionLow,
    MediaResolutionMedium,
    MediaResolutionHigh,
    MediaResolutionUltraHigh,
}

/// Safety filter level for image generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SafetyFilterLevel {
    BlockLowAndAbove,
    BlockMediumAndAbove,
    BlockOnlyHigh,
    BlockNone,
}

/// Person generation policy for media generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PersonGeneration {
    DontAllow,
    AllowAdult,
    AllowAll,
}

/// Prompt language for image generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImagePromptLanguage {
    Auto,
    En,
    Ja,
    Ko,
    Hi,
    Zh,
    Pt,
    Es,
}

/// Reference image type for image editing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReferenceImageType {
    ReferenceTypeRaw,
    ReferenceTypeMask,
    ReferenceTypeControl,
    ReferenceTypeStyle,
    ReferenceTypeSubject,
    ReferenceTypeContent,
}

/// Mask reference mode for image editing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MaskReferenceMode {
    MaskModeDefault,
    MaskModeUserProvided,
    MaskModeBackground,
    MaskModeForeground,
    MaskModeSemantic,
}

/// Control reference type for image editing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ControlReferenceType {
    ControlTypeDefault,
    ControlTypeCanny,
    ControlTypeScribble,
    ControlTypeFaceMesh,
}

/// Subject reference type for image editing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SubjectReferenceType {
    SubjectTypeDefault,
    SubjectTypePerson,
    SubjectTypeAnimal,
    SubjectTypeProduct,
}

/// Edit mode for image editing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EditMode {
    EditModeDefault,
    EditModeInpaintRemoval,
    EditModeInpaintInsertion,
    EditModeOutpaint,
    EditModeControlledEditing,
    EditModeStyle,
    EditModeBgswap,
    EditModeProductImage,
}

/// Segment image mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SegmentMode {
    Foreground,
    Background,
    Prompt,
    Semantic,
    Interactive,
}

/// Reference image type for video generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VideoGenerationReferenceType {
    Asset,
    Style,
}

/// Mask mode for video generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VideoGenerationMaskMode {
    Insert,
    Remove,
    RemoveStatic,
    Outpaint,
}

/// Video compression quality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VideoCompressionQuality {
    Optimized,
    Lossless,
}

/// The tokenization quality used for a media part (per-part).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PartMediaResolutionLevel {
    MediaResolutionUnspecified,
    MediaResolutionLow,
    MediaResolutionMedium,
    MediaResolutionHigh,
    MediaResolutionUltraHigh,
}

/// Function behavior for Live API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Behavior {
    Unspecified,
    Blocking,
    NonBlocking,
}

/// Function calling mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FunctionCallingMode {
    ModeUnspecified,
    Auto,
    Any,
    None,
    Validated,
}

/// Harm category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HarmCategory {
    HarmCategoryUnspecified,
    HarmCategoryHarassment,
    HarmCategoryHateSpeech,
    HarmCategorySexuallyExplicit,
    HarmCategoryDangerousContent,
    HarmCategoryCivicIntegrity,
    HarmCategoryImageHate,
    HarmCategoryImageDangerousContent,
    HarmCategoryImageHarassment,
    HarmCategoryImageSexuallyExplicit,
    HarmCategoryJailbreak,
}

/// Harm block threshold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HarmBlockThreshold {
    HarmBlockThresholdUnspecified,
    BlockNone,
    BlockOnlyHigh,
    BlockMediumAndAbove,
    BlockLowAndAbove,
    Off,
}

/// Harm probability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HarmProbability {
    HarmProbabilityUnspecified,
    Negligible,
    Low,
    Medium,
    High,
}

/// Specifies how the harm block threshold is applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HarmBlockMethod {
    HarmBlockMethodUnspecified,
    Severity,
    Probability,
}

/// Batch/Job state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum JobState {
    JobStateUnspecified,
    JobStateQueued,
    JobStatePending,
    JobStateRunning,
    JobStateSucceeded,
    JobStateFailed,
    JobStateCancelling,
    JobStateCancelled,
    JobStatePaused,
    JobStateExpired,
    JobStateUpdating,
    JobStatePartiallySucceeded,
}

/// Tuning method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TuningMethod {
    SupervisedFineTuning,
    PreferenceTuning,
    Distillation,
}

/// Tuning mode for SFT tuning (Vertex AI only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TuningMode {
    TuningModeUnspecified,
    TuningModeFull,
    TuningModePeftAdapter,
}

/// Adapter size for tuning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AdapterSize {
    AdapterSizeUnspecified,
    AdapterSizeOne,
    AdapterSizeTwo,
    AdapterSizeFour,
    AdapterSizeEight,
    AdapterSizeSixteen,
    AdapterSizeThirtyTwo,
}

/// Reason why the prompt was blocked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BlockedReason {
    BlockedReasonUnspecified,
    Safety,
    Other,
    Blocklist,
    ProhibitedContent,
    ImageSafety,
    ModelArmor,
    Jailbreak,
}

/// The reason why token generation stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FinishReason {
    FinishReasonUnspecified,
    Stop,
    MaxTokens,
    Safety,
    Recitation,
    Language,
    Other,
    Blocklist,
}

/// Thinking level for thinking models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ThinkingLevel {
    ThinkingLevelUnspecified,
    Low,
    Medium,
    High,
    Minimal,
}

/// Status of the URL retrieval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UrlRetrievalStatus {
    UrlRetrievalStatusUnspecified,
    UrlRetrievalStatusSuccess,
    UrlRetrievalStatusError,
    UrlRetrievalStatusPaywall,
    UrlRetrievalStatusUnsafe,
}

/// Harm severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HarmSeverity {
    HarmSeverityUnspecified,
    HarmSeverityNegligible,
    HarmSeverityLow,
    HarmSeverityMedium,
    HarmSeverityHigh,
}

/// Traffic type for quota accounting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TrafficType {
    TrafficTypeUnspecified,
    OnDemand,
    ProvisionedThroughput,
}

/// File state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FileState {
    #[serde(alias = "STATE_UNSPECIFIED")]
    StateUnspecified,
    #[serde(alias = "STATE_PROCESSING")]
    Processing,
    #[serde(alias = "STATE_ACTIVE")]
    Active,
    #[serde(alias = "STATE_FAILED")]
    Failed,
}

/// Document state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DocumentState {
    #[serde(alias = "STATE_UNSPECIFIED")]
    StateUnspecified,
    #[serde(alias = "STATE_PENDING")]
    Pending,
    #[serde(alias = "STATE_ACTIVE")]
    Active,
    #[serde(alias = "STATE_FAILED")]
    Failed,
}

/// File source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FileSource {
    SourceUnspecified,
    Uploaded,
    Generated,
}

/// Reason why the turn is complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TurnCompleteReason {
    TurnCompleteReasonUnspecified,
    MalformedFunctionCall,
    ResponseRejected,
    NeedMoreInput,
}

/// Dynamic retrieval mode for `GoogleSearchRetrieval`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DynamicRetrievalConfigMode {
    ModeUnspecified,
    ModeDynamic,
}

/// Environment for computer use tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Environment {
    EnvironmentUnspecified,
    EnvironmentBrowser,
}

/// Feature selection preference for model routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FeatureSelectionPreference {
    FeatureSelectionPreferenceUnspecified,
    PrioritizeQuality,
    Balanced,
    PrioritizeCost,
}

/// The API spec that the external API implements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApiSpec {
    ApiSpecUnspecified,
    SimpleSearch,
    ElasticSearch,
}

/// Type of auth scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuthType {
    AuthTypeUnspecified,
    NoAuth,
    ApiKeyAuth,
    HttpBasicAuth,
    GoogleServiceAccountAuth,
    Oauth,
    OidcAuth,
}

/// Location of the API key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HttpElementLocation {
    HttpInUnspecified,
    HttpInQuery,
    HttpInHeader,
}

/// Phishing block threshold for Google Search.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PhishBlockThreshold {
    PhishBlockThresholdUnspecified,
    BlockLowAndAbove,
    BlockMediumAndAbove,
    BlockHighAndAbove,
    BlockHigherAndAbove,
    BlockVeryHighAndAbove,
    BlockOnlyExtremelyHigh,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harm_category_serialization() {
        let value = serde_json::to_string(&HarmCategory::HarmCategoryDangerousContent).unwrap();
        assert_eq!(value, "\"HARM_CATEGORY_DANGEROUS_CONTENT\"");
    }

    #[test]
    fn image_prompt_language_serialization() {
        let value = serde_json::to_string(&ImagePromptLanguage::Zh).unwrap();
        assert_eq!(value, "\"zh\"");
    }
}
