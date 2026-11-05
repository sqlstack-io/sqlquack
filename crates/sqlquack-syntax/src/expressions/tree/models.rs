//! Data models of expressions tree.

use serde::Deserialize;
use serde::Serialize;
use std::fmt;

use crate::expressions::columns::models::AlterColumn;
use crate::expressions::columns::models::AlterSet;
use crate::expressions::columns::models::AlterSortKey;
use crate::expressions::columns::models::AlterTable;
use crate::expressions::columns::models::AssumeColumnConstraint;
use crate::expressions::columns::models::AutoIncrementColumnConstraint;
use crate::expressions::columns::models::CaseSpecificColumnConstraint;
use crate::expressions::columns::models::CharacterSetColumnConstraint;
use crate::expressions::columns::models::CheckColumnConstraint;
use crate::expressions::columns::models::ColumnConstraint;
use crate::expressions::columns::models::ColumnDef;
use crate::expressions::columns::models::ColumnPosition;
use crate::expressions::columns::models::CommentColumnConstraint;
use crate::expressions::columns::models::Comprehension;
use crate::expressions::columns::models::CompressColumnConstraint;
use crate::expressions::columns::models::ComputedColumnConstraint;
use crate::expressions::columns::models::Constraint;
use crate::expressions::columns::models::CreateDataMetricFunction;
use crate::expressions::columns::models::DateFormatColumnConstraint;
use crate::expressions::columns::models::DefaultColumnConstraint;
use crate::expressions::columns::models::EphemeralColumnConstraint;
use crate::expressions::columns::models::GeneratedAsIdentityColumnConstraint;
use crate::expressions::columns::models::GeneratedAsRowColumnConstraint;
use crate::expressions::columns::models::InOutColumnConstraint;
use crate::expressions::columns::models::IndexColumnConstraint;
use crate::expressions::columns::models::IndexConstraintOption;
use crate::expressions::columns::models::MaskingPolicyColumnConstraint;
use crate::expressions::columns::models::MergeTreeTTL;
use crate::expressions::columns::models::MergeTreeTTLAction;
use crate::expressions::columns::models::NotNullColumnConstraint;
use crate::expressions::columns::models::PathColumnConstraint;
use crate::expressions::columns::models::PeriodForSystemTimeConstraint;
use crate::expressions::columns::models::PrimaryKeyColumnConstraint;
use crate::expressions::columns::models::RenameColumn;
use crate::expressions::columns::models::UniqueColumnConstraint;
use crate::expressions::columns::models::WatermarkColumnConstraint;
use crate::expressions::columns::models::WithOperator;
use crate::expressions::functions::models::AIAgg;
use crate::expressions::functions::models::AIClassify;
use crate::expressions::functions::models::AggregateFunction;
use crate::expressions::functions::models::Aliases;
use crate::expressions::functions::models::Anonymous;
use crate::expressions::functions::models::AnonymousAggFunc;
use crate::expressions::functions::models::Apply;
use crate::expressions::functions::models::ApproxQuantiles;
use crate::expressions::functions::models::ApproxTopK;
use crate::expressions::functions::models::ApproxTopKAccumulate;
use crate::expressions::functions::models::ApproxTopKCombine;
use crate::expressions::functions::models::ApproxTopKEstimate;
use crate::expressions::functions::models::ApproxTopSum;
use crate::expressions::functions::models::ArgMax;
use crate::expressions::functions::models::ArgMin;
use crate::expressions::functions::models::ArrayAll;
use crate::expressions::functions::models::ArrayAny;
use crate::expressions::functions::models::ArrayConstructCompact;
use crate::expressions::functions::models::ArraySum;
use crate::expressions::functions::models::AtIndex;
use crate::expressions::functions::models::Base64DecodeBinary;
use crate::expressions::functions::models::Base64DecodeString;
use crate::expressions::functions::models::Base64Encode;
use crate::expressions::functions::models::Booland;
use crate::expressions::functions::models::Boolor;
use crate::expressions::functions::models::CastToStrType;
use crate::expressions::functions::models::CheckJson;
use crate::expressions::functions::models::CheckXml;
use crate::expressions::functions::models::Columns;
use crate::expressions::functions::models::CombinedAggFunc;
use crate::expressions::functions::models::CombinedParameterizedAgg;
use crate::expressions::functions::models::Compress;
use crate::expressions::functions::models::ConvertTimezone;
use crate::expressions::functions::models::ConvertToCharset;
use crate::expressions::functions::models::CosineDistance;
use crate::expressions::functions::models::CurrentDatetime;
use crate::expressions::functions::models::CurrentSchema;
use crate::expressions::functions::models::CurrentSchemas;
use crate::expressions::functions::models::CurrentUser;
use crate::expressions::functions::models::DPipe;
use crate::expressions::functions::models::DateBin;
use crate::expressions::functions::models::DateFromParts;
use crate::expressions::functions::models::Datetime;
use crate::expressions::functions::models::DatetimeAdd;
use crate::expressions::functions::models::DatetimeDiff;
use crate::expressions::functions::models::DatetimeSub;
use crate::expressions::functions::models::DatetimeTrunc;
use crate::expressions::functions::models::Dayname;
use crate::expressions::functions::models::DecodeCase;
use crate::expressions::functions::models::DecompressBinary;
use crate::expressions::functions::models::DecompressString;
use crate::expressions::functions::models::Decrypt;
use crate::expressions::functions::models::DecryptRaw;
use crate::expressions::functions::models::DotProduct;
use crate::expressions::functions::models::Elt;
use crate::expressions::functions::models::Encode;
use crate::expressions::functions::models::Encrypt;
use crate::expressions::functions::models::EncryptRaw;
use crate::expressions::functions::models::EqualNull;
use crate::expressions::functions::models::EuclideanDistance;
use crate::expressions::functions::models::FarmFingerprint;
use crate::expressions::functions::models::Float64;
use crate::expressions::functions::models::ForIn;
use crate::expressions::functions::models::Format;
use crate::expressions::functions::models::FormatPhrase;
use crate::expressions::functions::models::FromTimeZone;
use crate::expressions::functions::models::Function;
use crate::expressions::functions::models::GapFill;
use crate::expressions::functions::models::GenerateDateArray;
use crate::expressions::functions::models::GenerateSeries;
use crate::expressions::functions::models::GenerateTimestampArray;
use crate::expressions::functions::models::GetExtract;
use crate::expressions::functions::models::Getbit;
use crate::expressions::functions::models::Grouping;
use crate::expressions::functions::models::GroupingId;
use crate::expressions::functions::models::HashAgg;
use crate::expressions::functions::models::HavingMax;
use crate::expressions::functions::models::HexEncode;
use crate::expressions::functions::models::Hll;
use crate::expressions::functions::models::IntervalOp;
use crate::expressions::functions::models::JSONArray;
use crate::expressions::functions::models::JSONArrayAgg;
use crate::expressions::functions::models::JSONArrayAppend;
use crate::expressions::functions::models::JSONArrayContains;
use crate::expressions::functions::models::JSONArrayInsert;
use crate::expressions::functions::models::JSONBExists;
use crate::expressions::functions::models::JSONBExtractScalar;
use crate::expressions::functions::models::JSONBObjectAgg;
use crate::expressions::functions::models::JSONCast;
use crate::expressions::functions::models::JSONColumnDef;
use crate::expressions::functions::models::JSONExists;
use crate::expressions::functions::models::JSONExtract;
use crate::expressions::functions::models::JSONExtractArray;
use crate::expressions::functions::models::JSONExtractQuote;
use crate::expressions::functions::models::JSONExtractScalar;
use crate::expressions::functions::models::JSONFormat;
use crate::expressions::functions::models::JSONKeyValue;
use crate::expressions::functions::models::JSONKeys;
use crate::expressions::functions::models::JSONKeysAtDepth;
use crate::expressions::functions::models::JSONObject;
use crate::expressions::functions::models::JSONObjectAgg;
use crate::expressions::functions::models::JSONPath;
use crate::expressions::functions::models::JSONPathFilter;
use crate::expressions::functions::models::JSONPathKey;
use crate::expressions::functions::models::JSONPathRecursive;
use crate::expressions::functions::models::JSONPathRoot;
use crate::expressions::functions::models::JSONPathScript;
use crate::expressions::functions::models::JSONPathSelector;
use crate::expressions::functions::models::JSONPathSlice;
use crate::expressions::functions::models::JSONPathSubscript;
use crate::expressions::functions::models::JSONPathUnion;
use crate::expressions::functions::models::JSONRemove;
use crate::expressions::functions::models::JSONSchema;
use crate::expressions::functions::models::JSONSet;
use crate::expressions::functions::models::JSONStripNulls;
use crate::expressions::functions::models::JSONTable;
use crate::expressions::functions::models::JSONType;
use crate::expressions::functions::models::JSONValue;
use crate::expressions::functions::models::JSONValueArray;
use crate::expressions::functions::models::JarowinklerSimilarity;
use crate::expressions::functions::models::List;
use crate::expressions::functions::models::Localtime;
use crate::expressions::functions::models::Localtimestamp;
use crate::expressions::functions::models::MakeInterval;
use crate::expressions::functions::models::ManhattanDistance;
use crate::expressions::functions::models::Minhash;
use crate::expressions::functions::models::Nullif;
use crate::expressions::functions::models::ObjectAgg;
use crate::expressions::functions::models::ObjectInsert;
use crate::expressions::functions::models::OpenJSON;
use crate::expressions::functions::models::OpenJSONColumnDef;
use crate::expressions::functions::models::Operator;
use crate::expressions::functions::models::OverflowTruncateBehavior;
use crate::expressions::functions::models::Pad;
use crate::expressions::functions::models::ParameterizedAgg;
use crate::expressions::functions::models::ParseDatetime;
use crate::expressions::functions::models::ParseIp;
use crate::expressions::functions::models::ParseJSON;
use crate::expressions::functions::models::ParseTime;
use crate::expressions::functions::models::ParseUrl;
use crate::expressions::functions::models::PivotAny;
use crate::expressions::functions::models::PreviousDay;
use crate::expressions::functions::models::SessionUser;
use crate::expressions::functions::models::StDistance;
use crate::expressions::functions::models::StPoint;
use crate::expressions::functions::models::StringFunc;
use crate::expressions::functions::models::StringToArray;
use crate::expressions::functions::models::Systimestamp;
use crate::expressions::functions::models::TimeAdd;
use crate::expressions::functions::models::TimeDiff;
use crate::expressions::functions::models::TimeFromParts;
use crate::expressions::functions::models::TimeSlice;
use crate::expressions::functions::models::TimeSub;
use crate::expressions::functions::models::TimeTrunc;
use crate::expressions::functions::models::TimeUnit;
use crate::expressions::functions::models::TimestampAdd;
use crate::expressions::functions::models::TimestampDiff;
use crate::expressions::functions::models::TimestampFunc;
use crate::expressions::functions::models::TimestampSub;
use crate::expressions::functions::models::ToBinary;
use crate::expressions::functions::models::ToBoolean;
use crate::expressions::functions::models::ToChar;
use crate::expressions::functions::models::ToDecfloat;
use crate::expressions::functions::models::ToDouble;
use crate::expressions::functions::models::ToFile;
use crate::expressions::functions::models::ToMap;
use crate::expressions::functions::models::ToNumber;
use crate::expressions::functions::models::Transform;
use crate::expressions::functions::models::Translate;
use crate::expressions::functions::models::TranslateCharacters;
use crate::expressions::functions::models::TryBase64DecodeBinary;
use crate::expressions::functions::models::TryBase64DecodeString;
use crate::expressions::functions::models::TryToDecfloat;
use crate::expressions::functions::models::UtcTime;
use crate::expressions::functions::models::UtcTimestamp;
use crate::expressions::functions::models::WindowFunction;
use crate::expressions::functions::models::WithinGroup;
use crate::expressions::functions::models::XMLElement;
use crate::expressions::functions::models::XMLGet;
use crate::expressions::functions::models::XMLKeyValueOption;
use crate::expressions::functions::models::XMLTable;
use crate::expressions::functions::models::Xor;
use crate::expressions::functions::models::JSON;
use crate::expressions::names::models::BooleanLiteral;
use crate::expressions::names::models::ByteString;
use crate::expressions::names::models::Column;
use crate::expressions::names::models::DefaultKeyword;
use crate::expressions::names::models::HexStringExpr;
use crate::expressions::names::models::Identifier;
use crate::expressions::names::models::Literal;
use crate::expressions::names::models::Null;
use crate::expressions::names::models::Star;
use crate::expressions::names::models::TableRef;
use crate::expressions::names::models::UnicodeString;
use crate::expressions::operators::models::Alias;
use crate::expressions::operators::models::ArraySlice;
use crate::expressions::operators::models::Between;
use crate::expressions::operators::models::BinaryOp;
use crate::expressions::operators::models::Call;
use crate::expressions::operators::models::Case;
use crate::expressions::operators::models::Cast;
use crate::expressions::operators::models::Changes;
use crate::expressions::operators::models::CollationExpr;
use crate::expressions::operators::models::ColumnPrefix;
use crate::expressions::operators::models::ConditionalInsert;
use crate::expressions::operators::models::Cube;
use crate::expressions::operators::models::Directory;
use crate::expressions::operators::models::DotAccess;
use crate::expressions::operators::models::Exists;
use crate::expressions::operators::models::Export;
use crate::expressions::operators::models::Filter;
use crate::expressions::operators::models::ForeignKey;
use crate::expressions::operators::models::Group;
use crate::expressions::operators::models::GroupingSets;
use crate::expressions::operators::models::Hierarchy;
use crate::expressions::operators::models::In;
use crate::expressions::operators::models::Index;
use crate::expressions::operators::models::IndexParameters;
use crate::expressions::operators::models::IntoClause;
use crate::expressions::operators::models::Introducer;
use crate::expressions::operators::models::IsJson;
use crate::expressions::operators::models::IsNull;
use crate::expressions::operators::models::IsTrueFalse;
use crate::expressions::operators::models::JoinHint;
use crate::expressions::operators::models::LambdaExpr;
use crate::expressions::operators::models::Lateral;
use crate::expressions::operators::models::LikeOp;
use crate::expressions::operators::models::LimitOptions;
use crate::expressions::operators::models::MethodCall;
use crate::expressions::operators::models::MultitableInserts;
use crate::expressions::operators::models::NamedArgument;
use crate::expressions::operators::models::OnCondition;
use crate::expressions::operators::models::OnConflict;
use crate::expressions::operators::models::Opclass;
use crate::expressions::operators::models::OverlapsExpr;
use crate::expressions::operators::models::Parameter;
use crate::expressions::operators::models::PartitionRange;
use crate::expressions::operators::models::Placeholder;
use crate::expressions::operators::models::PrimaryKey;
use crate::expressions::operators::models::QuantifiedExpr;
use crate::expressions::operators::models::Returning;
use crate::expressions::operators::models::Rollup;
use crate::expressions::operators::models::RowsFrom;
use crate::expressions::operators::models::SimilarToExpr;
use crate::expressions::operators::models::SqlComment;
use crate::expressions::operators::models::StorageProperty;
use crate::expressions::operators::models::Subscript;
use crate::expressions::operators::models::TableArgument;
use crate::expressions::operators::models::TableFromRows;
use crate::expressions::operators::models::UnaryOp;
use crate::expressions::operators::models::Upsert;
use crate::expressions::operators::models::ViewParameter;
use crate::expressions::operators::models::WithFill;
use crate::expressions::queries::models::ClusterBy;
use crate::expressions::queries::models::Connect;
use crate::expressions::queries::models::ConnectByRoot;
use crate::expressions::queries::models::Cte;
use crate::expressions::queries::models::DistributeBy;
use crate::expressions::queries::models::Except;
use crate::expressions::queries::models::Fetch;
use crate::expressions::queries::models::From;
use crate::expressions::queries::models::GroupBy;
use crate::expressions::queries::models::Having;
use crate::expressions::queries::models::Hint;
use crate::expressions::queries::models::Intersect;
use crate::expressions::queries::models::Join;
use crate::expressions::queries::models::JoinedTable;
use crate::expressions::queries::models::LateralView;
use crate::expressions::queries::models::Limit;
use crate::expressions::queries::models::MatchRecognize;
use crate::expressions::queries::models::MatchRecognizeMeasure;
use crate::expressions::queries::models::Offset;
use crate::expressions::queries::models::OrderBy;
use crate::expressions::queries::models::Ordered;
use crate::expressions::queries::models::Over;
use crate::expressions::queries::models::PipeOperator;
use crate::expressions::queries::models::Pivot;
use crate::expressions::queries::models::PivotAlias;
use crate::expressions::queries::models::PreWhere;
use crate::expressions::queries::models::Prior;
use crate::expressions::queries::models::Pseudocolumn;
use crate::expressions::queries::models::Qualify;
use crate::expressions::queries::models::Resample;
use crate::expressions::queries::models::Select;
use crate::expressions::queries::models::SortBy;
use crate::expressions::queries::models::Stream;
use crate::expressions::queries::models::Subquery;
use crate::expressions::queries::models::Union;
use crate::expressions::queries::models::Unpivot;
use crate::expressions::queries::models::UsingData;
use crate::expressions::queries::models::Values;
use crate::expressions::queries::models::Where;
use crate::expressions::queries::models::WindowSpec;
use crate::expressions::queries::models::With;
use crate::expressions::queries::models::XmlNamespace;
use crate::expressions::schema_objects::models::AlterIndex;
use crate::expressions::schema_objects::models::AlterObject;
use crate::expressions::schema_objects::models::AlterSequence;
use crate::expressions::schema_objects::models::AlterView;
use crate::expressions::schema_objects::models::CreateDatabase;
use crate::expressions::schema_objects::models::CreateFunction;
use crate::expressions::schema_objects::models::CreateObject;
use crate::expressions::schema_objects::models::CreateProcedure;
use crate::expressions::schema_objects::models::CreateSchema;
use crate::expressions::schema_objects::models::CreateSequence;
use crate::expressions::schema_objects::models::CreateSynonym;
use crate::expressions::schema_objects::models::CreateTrigger;
use crate::expressions::schema_objects::models::CreateType;
use crate::expressions::schema_objects::models::CreateView;
use crate::expressions::schema_objects::models::DropDatabase;
use crate::expressions::schema_objects::models::DropFunction;
use crate::expressions::schema_objects::models::DropNamespace;
use crate::expressions::schema_objects::models::DropProcedure;
use crate::expressions::schema_objects::models::DropSchema;
use crate::expressions::schema_objects::models::DropSequence;
use crate::expressions::schema_objects::models::DropTrigger;
use crate::expressions::schema_objects::models::DropType;
use crate::expressions::schema_objects::models::DropView;
use crate::expressions::schema_objects::models::ExecuteObject;
use crate::expressions::statements::models::AddPartition;
use crate::expressions::statements::models::AlterSession;
use crate::expressions::statements::models::Analyze;
use crate::expressions::statements::models::AnalyzeDelete;
use crate::expressions::statements::models::AnalyzeHistogram;
use crate::expressions::statements::models::AnalyzeListChainedRows;
use crate::expressions::statements::models::AnalyzeSample;
use crate::expressions::statements::models::AnalyzeStatistics;
use crate::expressions::statements::models::AnalyzeValidate;
use crate::expressions::statements::models::AnalyzeWith;
use crate::expressions::statements::models::Annotated;
use crate::expressions::statements::models::Attach;
use crate::expressions::statements::models::AttachOption;
use crate::expressions::statements::models::Cache;
use crate::expressions::statements::models::Clone;
use crate::expressions::statements::models::Command;
use crate::expressions::statements::models::Comment;
use crate::expressions::statements::models::Commit;
use crate::expressions::statements::models::CopyParameter;
use crate::expressions::statements::models::CopyStmt;
use crate::expressions::statements::models::CreateTask;
use crate::expressions::statements::models::Credentials;
use crate::expressions::statements::models::Declare;
use crate::expressions::statements::models::DeclareItem;
use crate::expressions::statements::models::Delete;
use crate::expressions::statements::models::Describe;
use crate::expressions::statements::models::Detach;
use crate::expressions::statements::models::DropPartition;
use crate::expressions::statements::models::ExecuteStatement;
use crate::expressions::statements::models::ExecuteUsingPolicyContext;
use crate::expressions::statements::models::Get;
use crate::expressions::statements::models::Grant;
use crate::expressions::statements::models::GrantPrincipal;
use crate::expressions::statements::models::Heredoc;
use crate::expressions::statements::models::HistoricalData;
use crate::expressions::statements::models::Insert;
use crate::expressions::statements::models::Install;
use crate::expressions::statements::models::Kill;
use crate::expressions::statements::models::LoadData;
use crate::expressions::statements::models::Lock;
use crate::expressions::statements::models::LockingStatement;
use crate::expressions::statements::models::Merge;
use crate::expressions::statements::models::NextValueFor;
use crate::expressions::statements::models::ObjectIdentifier;
use crate::expressions::statements::models::Paren;
use crate::expressions::statements::models::Pragma;
use crate::expressions::statements::models::PrepareStatement;
use crate::expressions::statements::models::ProjectionDef;
use crate::expressions::statements::models::PseudoType;
use crate::expressions::statements::models::PutStmt;
use crate::expressions::statements::models::QueryBand;
use crate::expressions::statements::models::Raw;
use crate::expressions::statements::models::RecursiveWithSearch;
use crate::expressions::statements::models::Refresh;
use crate::expressions::statements::models::ReplacePartition;
use crate::expressions::statements::models::Revoke;
use crate::expressions::statements::models::Rollback;
use crate::expressions::statements::models::Schema;
use crate::expressions::statements::models::ScriptingBlock;
use crate::expressions::statements::models::SequenceProperties;
use crate::expressions::statements::models::SessionParameter;
use crate::expressions::statements::models::Set;
use crate::expressions::statements::models::SetItem;
use crate::expressions::statements::models::SetOperation;
use crate::expressions::statements::models::SetStatement;
use crate::expressions::statements::models::Show;
use crate::expressions::statements::models::StageReference;
use crate::expressions::statements::models::Summarize;
use crate::expressions::statements::models::TableAlias;
use crate::expressions::statements::models::TableSample;
use crate::expressions::statements::models::Tag;
use crate::expressions::statements::models::Transaction;
use crate::expressions::statements::models::Truncate;
use crate::expressions::statements::models::TruncateTable;
use crate::expressions::statements::models::TryCatch;
use crate::expressions::statements::models::Uncache;
use crate::expressions::statements::models::UnpivotColumns;
use crate::expressions::statements::models::UnsetStatement;
use crate::expressions::statements::models::Update;
use crate::expressions::statements::models::Use;
use crate::expressions::statements::models::UserDefinedFunction;
use crate::expressions::statements::models::Var;
use crate::expressions::statements::models::Variadic;
use crate::expressions::statements::models::Version;
use crate::expressions::statements::models::VerticaExpression;
use crate::expressions::statements::models::When;
use crate::expressions::statements::models::Whens;
use crate::expressions::tables::models::AlgorithmProperty;
use crate::expressions::tables::models::AllowedValuesProperty;
use crate::expressions::tables::models::AutoIncrementProperty;
use crate::expressions::tables::models::AutoRefreshProperty;
use crate::expressions::tables::models::BackupProperty;
use crate::expressions::tables::models::BlockCompressionProperty;
use crate::expressions::tables::models::BuildProperty;
use crate::expressions::tables::models::CharacterSetProperty;
use crate::expressions::tables::models::ChecksumProperty;
use crate::expressions::tables::models::ClusterByColumnsProperty;
use crate::expressions::tables::models::ClusteredByProperty;
use crate::expressions::tables::models::CollateProperty;
use crate::expressions::tables::models::CreateIndex;
use crate::expressions::tables::models::CreateTable;
use crate::expressions::tables::models::CredentialsProperty;
use crate::expressions::tables::models::DataBlocksizeProperty;
use crate::expressions::tables::models::DataDeletionProperty;
use crate::expressions::tables::models::DefinerProperty;
use crate::expressions::tables::models::DictProperty;
use crate::expressions::tables::models::DictRange;
use crate::expressions::tables::models::DistKeyProperty;
use crate::expressions::tables::models::DistStyleProperty;
use crate::expressions::tables::models::DistributedByProperty;
use crate::expressions::tables::models::DropIndex;
use crate::expressions::tables::models::DropObject;
use crate::expressions::tables::models::DropTable;
use crate::expressions::tables::models::DuplicateKeyProperty;
use crate::expressions::tables::models::EncodeProperty;
use crate::expressions::tables::models::EngineProperty;
use crate::expressions::tables::models::EnviromentProperty;
use crate::expressions::tables::models::ExecuteAsProperty;
use crate::expressions::tables::models::ExternalProperty;
use crate::expressions::tables::models::FallbackProperty;
use crate::expressions::tables::models::FileFormatProperty;
use crate::expressions::tables::models::FlashbackTable;
use crate::expressions::tables::models::FreespaceProperty;
use crate::expressions::tables::models::GrantPrivilege;
use crate::expressions::tables::models::IncludeProperty;
use crate::expressions::tables::models::IndexTableHint;
use crate::expressions::tables::models::InheritsProperty;
use crate::expressions::tables::models::InputModelProperty;
use crate::expressions::tables::models::InputOutputFormat;
use crate::expressions::tables::models::IsolatedLoadingProperty;
use crate::expressions::tables::models::JournalProperty;
use crate::expressions::tables::models::LanguageProperty;
use crate::expressions::tables::models::LikeProperty;
use crate::expressions::tables::models::LocationProperty;
use crate::expressions::tables::models::LockProperty;
use crate::expressions::tables::models::LockingProperty;
use crate::expressions::tables::models::LogProperty;
use crate::expressions::tables::models::MaterializedProperty;
use crate::expressions::tables::models::MergeBlockRatioProperty;
use crate::expressions::tables::models::OnCluster;
use crate::expressions::tables::models::OnCommitProperty;
use crate::expressions::tables::models::OnProperty;
use crate::expressions::tables::models::OptionsProperty;
use crate::expressions::tables::models::OutputModelProperty;
use crate::expressions::tables::models::Partition;
use crate::expressions::tables::models::PartitionBoundSpec;
use crate::expressions::tables::models::PartitionByListProperty;
use crate::expressions::tables::models::PartitionByProperty;
use crate::expressions::tables::models::PartitionByRangeProperty;
use crate::expressions::tables::models::PartitionByRangePropertyDynamic;
use crate::expressions::tables::models::PartitionByTruncate;
use crate::expressions::tables::models::PartitionList;
use crate::expressions::tables::models::PartitionedByBucket;
use crate::expressions::tables::models::PartitionedByProperty;
use crate::expressions::tables::models::PartitionedOfProperty;
use crate::expressions::tables::models::Properties;
use crate::expressions::tables::models::Property;
use crate::expressions::tables::models::QueryOption;
use crate::expressions::tables::models::QueryTransform;
use crate::expressions::tables::models::Reference;
use crate::expressions::tables::models::RefreshTriggerProperty;
use crate::expressions::tables::models::RemoteWithConnectionModelProperty;
use crate::expressions::tables::models::ReturnsProperty;
use crate::expressions::tables::models::RollupProperty;
use crate::expressions::tables::models::RowFormatDelimitedProperty;
use crate::expressions::tables::models::RowFormatProperty;
use crate::expressions::tables::models::RowFormatSerdeProperty;
use crate::expressions::tables::models::SampleProperty;
use crate::expressions::tables::models::SchemaCommentProperty;
use crate::expressions::tables::models::SecurityProperty;
use crate::expressions::tables::models::SemanticView;
use crate::expressions::tables::models::SerdeProperties;
use crate::expressions::tables::models::SetConfigProperty;
use crate::expressions::tables::models::SetProperty;
use crate::expressions::tables::models::SettingsProperty;
use crate::expressions::tables::models::SharingProperty;
use crate::expressions::tables::models::SortKeyProperty;
use crate::expressions::tables::models::SplitTable;
use crate::expressions::tables::models::SqlReadWriteProperty;
use crate::expressions::tables::models::SqlSecurityProperty;
use crate::expressions::tables::models::StabilityProperty;
use crate::expressions::tables::models::StorageHandlerProperty;
use crate::expressions::tables::models::Tags;
use crate::expressions::tables::models::TemporaryProperty;
use crate::expressions::tables::models::ToTableProperty;
use crate::expressions::tables::models::TransformModelProperty;
use crate::expressions::tables::models::TransientProperty;
use crate::expressions::tables::models::Undrop;
use crate::expressions::tables::models::UniqueKeyProperty;
use crate::expressions::tables::models::UsingTemplateProperty;
use crate::expressions::tables::models::ViewAttributeProperty;
use crate::expressions::tables::models::VolatileProperty;
use crate::expressions::tables::models::WithDataProperty;
use crate::expressions::tables::models::WithJournalTableProperty;
use crate::expressions::tables::models::WithProcedureOptions;
use crate::expressions::tables::models::WithSchemaBindingProperty;
use crate::expressions::tables::models::WithSystemVersioningProperty;
use crate::expressions::tables::models::WithTableHint;
use crate::expressions::typed_functions::models::AggFunc;
use crate::expressions::typed_functions::models::ApproxPercentileEstimate;
use crate::expressions::typed_functions::models::ApproxPercentileFunc;
use crate::expressions::typed_functions::models::ApproxQuantile;
use crate::expressions::typed_functions::models::ArrayConstructor;
use crate::expressions::typed_functions::models::ArrayFilterFunc;
use crate::expressions::typed_functions::models::ArrayJoinFunc;
use crate::expressions::typed_functions::models::ArraySortFunc;
use crate::expressions::typed_functions::models::ArrayTransformFunc;
use crate::expressions::typed_functions::models::AtTimeZone;
use crate::expressions::typed_functions::models::BinaryFunc;
use crate::expressions::typed_functions::models::CeilFunc;
use crate::expressions::typed_functions::models::CharFunc;
use crate::expressions::typed_functions::models::ConcatWs;
use crate::expressions::typed_functions::models::ConvertFunc;
use crate::expressions::typed_functions::models::Corr;
use crate::expressions::typed_functions::models::CountFunc;
use crate::expressions::typed_functions::models::CovarPop;
use crate::expressions::typed_functions::models::CovarSamp;
use crate::expressions::typed_functions::models::CumeDist;
use crate::expressions::typed_functions::models::CurrentDate;
use crate::expressions::typed_functions::models::CurrentTime;
use crate::expressions::typed_functions::models::CurrentTimestamp;
use crate::expressions::typed_functions::models::CurrentTimestampLTZ;
use crate::expressions::typed_functions::models::DateAddFunc;
use crate::expressions::typed_functions::models::DateDiffFunc;
use crate::expressions::typed_functions::models::DateFormatFunc;
use crate::expressions::typed_functions::models::DateTruncFunc;
use crate::expressions::typed_functions::models::DecodeFunc;
use crate::expressions::typed_functions::models::DenseRank;
use crate::expressions::typed_functions::models::ExtractFunc;
use crate::expressions::typed_functions::models::FeaturesAtTime;
use crate::expressions::typed_functions::models::FloorFunc;
use crate::expressions::typed_functions::models::FromBase;
use crate::expressions::typed_functions::models::FromUnixtimeFunc;
use crate::expressions::typed_functions::models::FunctionEmits;
use crate::expressions::typed_functions::models::GenerateEmbedding;
use crate::expressions::typed_functions::models::GroupConcatFunc;
use crate::expressions::typed_functions::models::IfFunc;
use crate::expressions::typed_functions::models::JsonArrayAggFunc;
use crate::expressions::typed_functions::models::JsonExtractFunc;
use crate::expressions::typed_functions::models::JsonModifyFunc;
use crate::expressions::typed_functions::models::JsonObjectAggFunc;
use crate::expressions::typed_functions::models::JsonObjectFunc;
use crate::expressions::typed_functions::models::JsonPathFunc;
use crate::expressions::typed_functions::models::LastDayFunc;
use crate::expressions::typed_functions::models::LeadLagFunc;
use crate::expressions::typed_functions::models::LeftRightFunc;
use crate::expressions::typed_functions::models::ListAggFunc;
use crate::expressions::typed_functions::models::LogFunc;
use crate::expressions::typed_functions::models::MD5Digest;
use crate::expressions::typed_functions::models::MLForecast;
use crate::expressions::typed_functions::models::MLTranslate;
use crate::expressions::typed_functions::models::MakeDateFunc;
use crate::expressions::typed_functions::models::MakeTimestampFunc;
use crate::expressions::typed_functions::models::Map;
use crate::expressions::typed_functions::models::MapCat;
use crate::expressions::typed_functions::models::MapConstructor;
use crate::expressions::typed_functions::models::MapDelete;
use crate::expressions::typed_functions::models::MapInsert;
use crate::expressions::typed_functions::models::MapPick;
use crate::expressions::typed_functions::models::MatchAgainst;
use crate::expressions::typed_functions::models::ModelAttribute;
use crate::expressions::typed_functions::models::Monthname;
use crate::expressions::typed_functions::models::NTileFunc;
use crate::expressions::typed_functions::models::NamedStructFunc;
use crate::expressions::typed_functions::models::Normal;
use crate::expressions::typed_functions::models::Normalize;
use crate::expressions::typed_functions::models::NthValueFunc;
use crate::expressions::typed_functions::models::Ntile;
use crate::expressions::typed_functions::models::NumberToStr;
use crate::expressions::typed_functions::models::Nvl2Func;
use crate::expressions::typed_functions::models::OverlayFunc;
use crate::expressions::typed_functions::models::PadFunc;
use crate::expressions::typed_functions::models::PercentRank;
use crate::expressions::typed_functions::models::PercentileFunc;
use crate::expressions::typed_functions::models::Pi;
use crate::expressions::typed_functions::models::PositionFunc;
use crate::expressions::typed_functions::models::Predict;
use crate::expressions::typed_functions::models::Quantile;
use crate::expressions::typed_functions::models::Rand;
use crate::expressions::typed_functions::models::Randn;
use crate::expressions::typed_functions::models::Random;
use crate::expressions::typed_functions::models::Randstr;
use crate::expressions::typed_functions::models::RangeBucket;
use crate::expressions::typed_functions::models::RangeN;
use crate::expressions::typed_functions::models::Rank;
use crate::expressions::typed_functions::models::ReadCSV;
use crate::expressions::typed_functions::models::ReadParquet;
use crate::expressions::typed_functions::models::Reduce;
use crate::expressions::typed_functions::models::RegexpCount;
use crate::expressions::typed_functions::models::RegexpExtractAll;
use crate::expressions::typed_functions::models::RegexpExtractFunc;
use crate::expressions::typed_functions::models::RegexpFullMatch;
use crate::expressions::typed_functions::models::RegexpFunc;
use crate::expressions::typed_functions::models::RegexpILike;
use crate::expressions::typed_functions::models::RegexpInstr;
use crate::expressions::typed_functions::models::RegexpReplaceFunc;
use crate::expressions::typed_functions::models::RegexpSplit;
use crate::expressions::typed_functions::models::RegrAvgx;
use crate::expressions::typed_functions::models::RegrAvgy;
use crate::expressions::typed_functions::models::RegrCount;
use crate::expressions::typed_functions::models::RegrIntercept;
use crate::expressions::typed_functions::models::RegrR2;
use crate::expressions::typed_functions::models::RegrSlope;
use crate::expressions::typed_functions::models::RegrSxx;
use crate::expressions::typed_functions::models::RegrSxy;
use crate::expressions::typed_functions::models::RegrSyy;
use crate::expressions::typed_functions::models::RegrValx;
use crate::expressions::typed_functions::models::RegrValy;
use crate::expressions::typed_functions::models::RepeatFunc;
use crate::expressions::typed_functions::models::ReplaceFunc;
use crate::expressions::typed_functions::models::RoundFunc;
use crate::expressions::typed_functions::models::RowNumber;
use crate::expressions::typed_functions::models::SHA2Digest;
use crate::expressions::typed_functions::models::SafeAdd;
use crate::expressions::typed_functions::models::SafeDivide;
use crate::expressions::typed_functions::models::SafeMultiply;
use crate::expressions::typed_functions::models::SafeSubtract;
use crate::expressions::typed_functions::models::ScopeResolution;
use crate::expressions::typed_functions::models::Search;
use crate::expressions::typed_functions::models::SearchIp;
use crate::expressions::typed_functions::models::SequenceFunc;
use crate::expressions::typed_functions::models::Slice;
use crate::expressions::typed_functions::models::SortArray;
use crate::expressions::typed_functions::models::SplitFunc;
use crate::expressions::typed_functions::models::SplitPart;
use crate::expressions::typed_functions::models::StandardHash;
use crate::expressions::typed_functions::models::StrPosition;
use crate::expressions::typed_functions::models::StrToDate;
use crate::expressions::typed_functions::models::StrToMap;
use crate::expressions::typed_functions::models::StrToTime;
use crate::expressions::typed_functions::models::StrToUnix;
use crate::expressions::typed_functions::models::StringAggFunc;
use crate::expressions::typed_functions::models::StructConstructor;
use crate::expressions::typed_functions::models::StructExtractFunc;
use crate::expressions::typed_functions::models::Stuff;
use crate::expressions::typed_functions::models::SubstringFunc;
use crate::expressions::typed_functions::models::SubstringIndex;
use crate::expressions::typed_functions::models::SumIfFunc;
use crate::expressions::typed_functions::models::TimeStrToTime;
use crate::expressions::typed_functions::models::TimeToStr;
use crate::expressions::typed_functions::models::TimestampFromParts;
use crate::expressions::typed_functions::models::TimestampTzFromParts;
use crate::expressions::typed_functions::models::ToDateFunc;
use crate::expressions::typed_functions::models::ToTimestampFunc;
use crate::expressions::typed_functions::models::TransformFunc;
use crate::expressions::typed_functions::models::TrimFunc;
use crate::expressions::typed_functions::models::TruncateFunc;
use crate::expressions::typed_functions::models::TsOrDsAdd;
use crate::expressions::typed_functions::models::TsOrDsDiff;
use crate::expressions::typed_functions::models::TsOrDsToDate;
use crate::expressions::typed_functions::models::TsOrDsToTime;
use crate::expressions::typed_functions::models::UnaryFunc;
use crate::expressions::typed_functions::models::Unhex;
use crate::expressions::typed_functions::models::Uniform;
use crate::expressions::typed_functions::models::UnixTimestampFunc;
use crate::expressions::typed_functions::models::UnixToStr;
use crate::expressions::typed_functions::models::UnixToTime;
use crate::expressions::typed_functions::models::UnnestFunc;
use crate::expressions::typed_functions::models::Uuid;
use crate::expressions::typed_functions::models::ValueFunc;
use crate::expressions::typed_functions::models::VarArgFunc;
use crate::expressions::typed_functions::models::VarMap;
use crate::expressions::typed_functions::models::VectorSearch;
use crate::expressions::typed_functions::models::Week;
use crate::expressions::typed_functions::models::WidthBucket;
use crate::expressions::typed_functions::models::Zipf;
use crate::expressions::typed_functions::models::SHA2;
use crate::expressions::types::models::Array;
use crate::expressions::types::models::DataType;
use crate::expressions::types::models::Interval;
use crate::expressions::types::models::IntervalSpan;
use crate::expressions::types::models::Struct;
use crate::expressions::types::models::Tuple;

/// Any SQL expression or statement, as one recursive AST node.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Expression {
    /// Structured Vertica syntax whose semantics are not shared by other dialects.
    Vertica(Box<VerticaExpression>),
    Literal(Box<Literal>),
    Boolean(BooleanLiteral),
    Null(Null),
    DefaultKeyword(DefaultKeyword),

    Identifier(Identifier),
    Column(Box<Column>),
    Table(Box<TableRef>),
    Star(Star),
    /// Snowflake braced wildcard syntax: {*}, {tbl.*}, {* EXCLUDE (...)}, {* ILIKE '...'}
    BracedWildcard(Box<Expression>),

    Select(Box<Select>),
    Union(Box<Union>),
    Intersect(Box<Intersect>),
    Except(Box<Except>),
    Subquery(Box<Subquery>),
    PipeOperator(Box<PipeOperator>),
    Pivot(Box<Pivot>),
    PivotAlias(Box<PivotAlias>),
    Unpivot(Box<Unpivot>),
    Values(Box<Values>),
    PreWhere(Box<PreWhere>),
    Stream(Box<Stream>),
    UsingData(Box<UsingData>),
    XmlNamespace(Box<XmlNamespace>),

    Insert(Box<Insert>),
    Update(Box<Update>),
    Delete(Box<Delete>),
    Copy(Box<CopyStmt>),
    Put(Box<PutStmt>),
    StageReference(Box<StageReference>),
    TryCatch(Box<TryCatch>),

    Alias(Box<Alias>),
    Cast(Box<Cast>),
    Collation(Box<CollationExpr>),
    Case(Box<Case>),

    And(Box<BinaryOp>),
    Or(Box<BinaryOp>),
    Add(Box<BinaryOp>),
    Sub(Box<BinaryOp>),
    Mul(Box<BinaryOp>),
    Div(Box<BinaryOp>),
    Mod(Box<BinaryOp>),
    Eq(Box<BinaryOp>),
    Neq(Box<BinaryOp>),
    Lt(Box<BinaryOp>),
    Lte(Box<BinaryOp>),
    Gt(Box<BinaryOp>),
    Gte(Box<BinaryOp>),
    Like(Box<LikeOp>),
    ILike(Box<LikeOp>),
    /// SQLite MATCH operator (FTS)
    Match(Box<BinaryOp>),
    BitwiseAnd(Box<BinaryOp>),
    BitwiseOr(Box<BinaryOp>),
    BitwiseXor(Box<BinaryOp>),
    Concat(Box<BinaryOp>),
    Adjacent(Box<BinaryOp>),
    TsMatch(Box<BinaryOp>),
    PropertyEQ(Box<BinaryOp>),

    ArrayContainsAll(Box<BinaryOp>),
    ArrayContainedBy(Box<BinaryOp>),
    ArrayOverlaps(Box<BinaryOp>),
    JSONBContainsAllTopKeys(Box<BinaryOp>),
    JSONBContainsAnyTopKeys(Box<BinaryOp>),
    JSONBDeleteAtPath(Box<BinaryOp>),
    ExtendsLeft(Box<BinaryOp>),
    ExtendsRight(Box<BinaryOp>),

    Not(Box<UnaryOp>),
    Neg(Box<UnaryOp>),
    /// Unary `+`, kept so the tree records the operator as written.
    Pos(Box<UnaryOp>),
    BitwiseNot(Box<UnaryOp>),

    In(Box<In>),
    Between(Box<Between>),
    IsNull(Box<IsNull>),
    IsTrue(Box<IsTrueFalse>),
    IsFalse(Box<IsTrueFalse>),
    IsJson(Box<IsJson>),
    Is(Box<BinaryOp>),
    Exists(Box<Exists>),
    /// MySQL MEMBER OF operator: expr MEMBER OF(json_array)
    MemberOf(Box<BinaryOp>),

    Function(Box<Function>),

    Upsert(Box<Upsert>),
    StorageProperty(Box<StorageProperty>),
    Hierarchy(Box<Hierarchy>),
    ViewParameter(Box<ViewParameter>),
    Call(Box<Call>),

    AggregateFunction(Box<AggregateFunction>),
    WindowFunction(Box<WindowFunction>),

    From(Box<From>),
    Join(Box<Join>),
    JoinedTable(Box<JoinedTable>),
    Where(Box<Where>),
    GroupBy(Box<GroupBy>),
    Having(Box<Having>),
    OrderBy(Box<OrderBy>),
    Limit(Box<Limit>),
    Offset(Box<Offset>),
    Qualify(Box<Qualify>),
    With(Box<With>),
    Cte(Box<Cte>),
    DistributeBy(Box<DistributeBy>),
    ClusterBy(Box<ClusterBy>),
    SortBy(Box<SortBy>),
    LateralView(Box<LateralView>),
    Hint(Box<Hint>),
    Pseudocolumn(Pseudocolumn),

    Connect(Box<Connect>),
    Prior(Box<Prior>),
    ConnectByRoot(Box<ConnectByRoot>),

    MatchRecognize(Box<MatchRecognize>),
    Resample(Box<Resample>),

    Ordered(Box<Ordered>),

    Window(Box<WindowSpec>),
    Over(Box<Over>),
    WithinGroup(Box<WithinGroup>),

    DataType(DataType),

    Array(Box<Array>),
    Struct(Box<Struct>),
    Tuple(Box<Tuple>),

    Interval(Box<Interval>),

    ConcatWs(Box<ConcatWs>),
    Substring(Box<SubstringFunc>),
    Upper(Box<UnaryFunc>),
    Lower(Box<UnaryFunc>),
    Length(Box<UnaryFunc>),
    Trim(Box<TrimFunc>),
    LTrim(Box<UnaryFunc>),
    RTrim(Box<UnaryFunc>),
    Replace(Box<ReplaceFunc>),
    Reverse(Box<UnaryFunc>),
    Left(Box<LeftRightFunc>),
    Right(Box<LeftRightFunc>),
    Repeat(Box<RepeatFunc>),
    Lpad(Box<PadFunc>),
    Rpad(Box<PadFunc>),
    Split(Box<SplitFunc>),
    RegexpLike(Box<RegexpFunc>),
    RegexpReplace(Box<RegexpReplaceFunc>),
    RegexpExtract(Box<RegexpExtractFunc>),
    Overlay(Box<OverlayFunc>),

    Abs(Box<UnaryFunc>),
    Round(Box<RoundFunc>),
    Floor(Box<FloorFunc>),
    Ceil(Box<CeilFunc>),
    Power(Box<BinaryFunc>),
    Sqrt(Box<UnaryFunc>),
    Cbrt(Box<UnaryFunc>),
    Ln(Box<UnaryFunc>),
    Log(Box<LogFunc>),
    Exp(Box<UnaryFunc>),
    Sign(Box<UnaryFunc>),
    Greatest(Box<VarArgFunc>),
    Least(Box<VarArgFunc>),

    CurrentDate(CurrentDate),
    CurrentTime(CurrentTime),
    CurrentTimestamp(CurrentTimestamp),
    CurrentTimestampLTZ(CurrentTimestampLTZ),
    AtTimeZone(Box<AtTimeZone>),
    DateAdd(Box<DateAddFunc>),
    DateSub(Box<DateAddFunc>),
    DateDiff(Box<DateDiffFunc>),
    DateTrunc(Box<DateTruncFunc>),
    Extract(Box<ExtractFunc>),
    ToDate(Box<ToDateFunc>),
    ToTimestamp(Box<ToTimestampFunc>),
    Date(Box<UnaryFunc>),
    Time(Box<UnaryFunc>),
    DateFromUnixDate(Box<UnaryFunc>),
    UnixDate(Box<UnaryFunc>),
    UnixSeconds(Box<UnaryFunc>),
    UnixMillis(Box<UnaryFunc>),
    UnixMicros(Box<UnaryFunc>),
    UnixToTimeStr(Box<BinaryFunc>),
    TimeStrToDate(Box<UnaryFunc>),
    DateToDi(Box<UnaryFunc>),
    DiToDate(Box<UnaryFunc>),
    TsOrDiToDi(Box<UnaryFunc>),
    TsOrDsToDatetime(Box<UnaryFunc>),
    TsOrDsToTimestamp(Box<UnaryFunc>),
    YearOfWeek(Box<UnaryFunc>),
    YearOfWeekIso(Box<UnaryFunc>),

    Coalesce(Box<VarArgFunc>),
    NullIf(Box<BinaryFunc>),
    IfFunc(Box<IfFunc>),
    IfNull(Box<BinaryFunc>),
    Nvl(Box<BinaryFunc>),
    Nvl2(Box<Nvl2Func>),

    TryCast(Box<Cast>),
    SafeCast(Box<Cast>),

    Count(Box<CountFunc>),
    Sum(Box<AggFunc>),
    Avg(Box<AggFunc>),
    Min(Box<AggFunc>),
    Max(Box<AggFunc>),
    GroupConcat(Box<GroupConcatFunc>),
    StringAgg(Box<StringAggFunc>),
    ListAgg(Box<ListAggFunc>),
    ArrayAgg(Box<AggFunc>),
    CountIf(Box<AggFunc>),
    SumIf(Box<SumIfFunc>),
    Stddev(Box<AggFunc>),
    StddevPop(Box<AggFunc>),
    StddevSamp(Box<AggFunc>),
    Variance(Box<AggFunc>),
    VarPop(Box<AggFunc>),
    VarSamp(Box<AggFunc>),
    Median(Box<AggFunc>),
    Mode(Box<AggFunc>),
    First(Box<AggFunc>),
    Last(Box<AggFunc>),
    AnyValue(Box<AggFunc>),
    ApproxDistinct(Box<AggFunc>),
    ApproxCountDistinct(Box<AggFunc>),
    ApproxPercentile(Box<ApproxPercentileFunc>),
    Percentile(Box<PercentileFunc>),
    LogicalAnd(Box<AggFunc>),
    LogicalOr(Box<AggFunc>),
    Skewness(Box<AggFunc>),
    BitwiseCount(Box<UnaryFunc>),
    ArrayConcatAgg(Box<AggFunc>),
    ArrayUniqueAgg(Box<AggFunc>),
    BoolXorAgg(Box<AggFunc>),

    RowNumber(RowNumber),
    Rank(Rank),
    DenseRank(DenseRank),
    NTile(Box<NTileFunc>),
    Lead(Box<LeadLagFunc>),
    Lag(Box<LeadLagFunc>),
    FirstValue(Box<ValueFunc>),
    LastValue(Box<ValueFunc>),
    NthValue(Box<NthValueFunc>),
    PercentRank(PercentRank),
    CumeDist(CumeDist),
    PercentileCont(Box<PercentileFunc>),
    PercentileDisc(Box<PercentileFunc>),

    Contains(Box<BinaryFunc>),
    StartsWith(Box<BinaryFunc>),
    EndsWith(Box<BinaryFunc>),
    Position(Box<PositionFunc>),
    Initcap(Box<UnaryFunc>),
    Ascii(Box<UnaryFunc>),
    Chr(Box<UnaryFunc>),
    /// MySQL CHAR function with multiple args and optional USING charset
    CharFunc(Box<CharFunc>),
    Soundex(Box<UnaryFunc>),
    Levenshtein(Box<BinaryFunc>),
    ByteLength(Box<UnaryFunc>),
    Hex(Box<UnaryFunc>),
    LowerHex(Box<UnaryFunc>),
    Unicode(Box<UnaryFunc>),

    ModFunc(Box<BinaryFunc>),
    Random(Random),
    Rand(Box<Rand>),
    TruncFunc(Box<TruncateFunc>),
    Pi(Pi),
    Radians(Box<UnaryFunc>),
    Degrees(Box<UnaryFunc>),
    Sin(Box<UnaryFunc>),
    Cos(Box<UnaryFunc>),
    Tan(Box<UnaryFunc>),
    Asin(Box<UnaryFunc>),
    Acos(Box<UnaryFunc>),
    Atan(Box<UnaryFunc>),
    Atan2(Box<BinaryFunc>),
    IsNan(Box<UnaryFunc>),
    IsInf(Box<UnaryFunc>),
    IntDiv(Box<BinaryFunc>),

    Decode(Box<DecodeFunc>),

    DateFormat(Box<DateFormatFunc>),
    FormatDate(Box<DateFormatFunc>),
    Year(Box<UnaryFunc>),
    Month(Box<UnaryFunc>),
    Day(Box<UnaryFunc>),
    Hour(Box<UnaryFunc>),
    Minute(Box<UnaryFunc>),
    Second(Box<UnaryFunc>),
    DayOfWeek(Box<UnaryFunc>),
    DayOfWeekIso(Box<UnaryFunc>),
    DayOfMonth(Box<UnaryFunc>),
    DayOfYear(Box<UnaryFunc>),
    WeekOfYear(Box<UnaryFunc>),
    Quarter(Box<UnaryFunc>),
    AddMonths(Box<BinaryFunc>),
    MonthsBetween(Box<BinaryFunc>),
    LastDay(Box<LastDayFunc>),
    NextDay(Box<BinaryFunc>),
    Epoch(Box<UnaryFunc>),
    EpochMs(Box<UnaryFunc>),
    FromUnixtime(Box<FromUnixtimeFunc>),
    UnixTimestamp(Box<UnixTimestampFunc>),
    MakeDate(Box<MakeDateFunc>),
    MakeTimestamp(Box<MakeTimestampFunc>),
    TimestampTrunc(Box<DateTruncFunc>),
    TimeStrToUnix(Box<UnaryFunc>),

    SessionUser(SessionUser),

    SHA(Box<UnaryFunc>),
    SHA1Digest(Box<UnaryFunc>),

    TimeToUnix(Box<UnaryFunc>),

    ArrayFunc(Box<ArrayConstructor>),
    ArrayLength(Box<UnaryFunc>),
    ArraySize(Box<UnaryFunc>),
    Cardinality(Box<UnaryFunc>),
    ArrayContains(Box<BinaryFunc>),
    ArrayPosition(Box<BinaryFunc>),
    ArrayAppend(Box<BinaryFunc>),
    ArrayPrepend(Box<BinaryFunc>),
    ArrayConcat(Box<VarArgFunc>),
    ArraySort(Box<ArraySortFunc>),
    ArrayReverse(Box<UnaryFunc>),
    ArrayDistinct(Box<UnaryFunc>),
    ArrayJoin(Box<ArrayJoinFunc>),
    ArrayToString(Box<ArrayJoinFunc>),
    Unnest(Box<UnnestFunc>),
    Explode(Box<UnaryFunc>),
    ExplodeOuter(Box<UnaryFunc>),
    ArrayFilter(Box<ArrayFilterFunc>),
    ArrayTransform(Box<ArrayTransformFunc>),
    ArrayFlatten(Box<UnaryFunc>),
    ArrayCompact(Box<UnaryFunc>),
    ArrayIntersect(Box<VarArgFunc>),
    ArrayUnion(Box<BinaryFunc>),
    ArrayExcept(Box<BinaryFunc>),
    ArrayRemove(Box<BinaryFunc>),
    ArrayZip(Box<VarArgFunc>),
    Sequence(Box<SequenceFunc>),
    Generate(Box<SequenceFunc>),
    ExplodingGenerateSeries(Box<SequenceFunc>),
    ToArray(Box<UnaryFunc>),
    StarMap(Box<BinaryFunc>),

    StructFunc(Box<StructConstructor>),
    StructExtract(Box<StructExtractFunc>),
    NamedStruct(Box<NamedStructFunc>),

    MapFunc(Box<MapConstructor>),
    MapFromEntries(Box<UnaryFunc>),
    MapFromArrays(Box<BinaryFunc>),
    MapKeys(Box<UnaryFunc>),
    MapValues(Box<UnaryFunc>),
    MapContainsKey(Box<BinaryFunc>),
    MapConcat(Box<VarArgFunc>),
    ElementAt(Box<BinaryFunc>),
    TransformKeys(Box<TransformFunc>),
    TransformValues(Box<TransformFunc>),

    FunctionEmits(Box<FunctionEmits>),

    JsonExtract(Box<JsonExtractFunc>),
    JsonExtractScalar(Box<JsonExtractFunc>),
    JsonExtractPath(Box<JsonPathFunc>),
    JsonArray(Box<VarArgFunc>),
    JsonObject(Box<JsonObjectFunc>),
    JsonQuery(Box<JsonExtractFunc>),
    JsonValue(Box<JsonExtractFunc>),
    JsonArrayLength(Box<UnaryFunc>),
    JsonKeys(Box<UnaryFunc>),
    JsonType(Box<UnaryFunc>),
    ParseJson(Box<UnaryFunc>),
    ToJson(Box<UnaryFunc>),
    JsonSet(Box<JsonModifyFunc>),
    JsonInsert(Box<JsonModifyFunc>),
    JsonRemove(Box<JsonPathFunc>),
    JsonMergePatch(Box<BinaryFunc>),
    JsonArrayAgg(Box<JsonArrayAggFunc>),
    JsonObjectAgg(Box<JsonObjectAggFunc>),

    Convert(Box<ConvertFunc>),
    Typeof(Box<UnaryFunc>),

    Lambda(Box<LambdaExpr>),
    Parameter(Box<Parameter>),
    Placeholder(Placeholder),
    NamedArgument(Box<NamedArgument>),
    /// TABLE or MODEL reference used as a function argument (BigQuery), as in GAP_FILL(TABLE t).
    TableArgument(Box<TableArgument>),
    SqlComment(Box<SqlComment>),

    NullSafeEq(Box<BinaryOp>),
    NullSafeNeq(Box<BinaryOp>),
    Glob(Box<BinaryOp>),
    SimilarTo(Box<SimilarToExpr>),
    Any(Box<QuantifiedExpr>),
    All(Box<QuantifiedExpr>),
    Overlaps(Box<OverlapsExpr>),

    BitwiseLeftShift(Box<BinaryOp>),
    BitwiseRightShift(Box<BinaryOp>),
    BitwiseAndAgg(Box<AggFunc>),
    BitwiseOrAgg(Box<AggFunc>),
    BitwiseXorAgg(Box<AggFunc>),

    Subscript(Box<Subscript>),
    Dot(Box<DotAccess>),
    MethodCall(Box<MethodCall>),
    ArraySlice(Box<ArraySlice>),

    CreateTable(Box<CreateTable>),
    DropTable(Box<DropTable>),
    Undrop(Box<Undrop>),
    DropObject(Box<DropObject>),
    AlterTable(Box<AlterTable>),
    SplitTable(Box<SplitTable>),
    FlashbackTable(Box<FlashbackTable>),
    CreateIndex(Box<CreateIndex>),
    DropIndex(Box<DropIndex>),
    CreateView(Box<CreateView>),
    DropView(Box<DropView>),
    AlterView(Box<AlterView>),
    AlterObject(Box<AlterObject>),
    CreateObject(Box<CreateObject>),
    CreateDataMetricFunction(Box<CreateDataMetricFunction>),
    AlterIndex(Box<AlterIndex>),
    Truncate(Box<Truncate>),
    Use(Box<Use>),
    Cache(Box<Cache>),
    Uncache(Box<Uncache>),
    LoadData(Box<LoadData>),
    Pragma(Box<Pragma>),
    Grant(Box<Grant>),
    Revoke(Box<Revoke>),
    Comment(Box<Comment>),
    SetStatement(Box<SetStatement>),
    UnsetStatement(Box<UnsetStatement>),
    CreateSchema(Box<CreateSchema>),
    DropSchema(Box<DropSchema>),
    DropNamespace(Box<DropNamespace>),
    CreateDatabase(Box<CreateDatabase>),
    DropDatabase(Box<DropDatabase>),
    CreateFunction(Box<CreateFunction>),
    DropFunction(Box<DropFunction>),
    CreateProcedure(Box<CreateProcedure>),
    DropProcedure(Box<DropProcedure>),
    CreateSequence(Box<CreateSequence>),
    CreateSynonym(Box<CreateSynonym>),
    DropSequence(Box<DropSequence>),
    AlterSequence(Box<AlterSequence>),
    CreateTrigger(Box<CreateTrigger>),
    DropTrigger(Box<DropTrigger>),
    CreateType(Box<CreateType>),
    DropType(Box<DropType>),
    Describe(Box<Describe>),
    Show(Box<Show>),

    Command(Box<Command>),
    ScriptingBlock(Box<ScriptingBlock>),
    Kill(Box<Kill>),
    /// PREPARE statement (PostgreSQL/generic prepared statement definition)
    Prepare(Box<PrepareStatement>),
    /// EXEC/EXECUTE statement (TSQL stored procedure call)
    Execute(Box<ExecuteStatement>),
    ExecuteObject(Box<ExecuteObject>),
    ExecuteUsingPolicyContext(Box<ExecuteUsingPolicyContext>),

    /// Snowflake CREATE TASK statement
    CreateTask(Box<CreateTask>),

    Raw(Raw),

    Paren(Box<Paren>),

    Annotated(Box<Annotated>),

    Refresh(Box<Refresh>),
    LockingStatement(Box<LockingStatement>),
    SequenceProperties(Box<SequenceProperties>),
    TruncateTable(Box<TruncateTable>),
    Clone(Box<Clone>),
    Attach(Box<Attach>),
    Detach(Box<Detach>),
    Install(Box<Install>),
    Summarize(Box<Summarize>),
    Declare(Box<Declare>),
    DeclareItem(Box<DeclareItem>),
    Set(Box<Set>),
    Heredoc(Box<Heredoc>),
    SetItem(Box<SetItem>),
    QueryBand(Box<QueryBand>),
    UserDefinedFunction(Box<UserDefinedFunction>),
    RecursiveWithSearch(Box<RecursiveWithSearch>),
    ProjectionDef(Box<ProjectionDef>),
    TableAlias(Box<TableAlias>),
    ByteString(Box<ByteString>),
    HexStringExpr(Box<HexStringExpr>),
    UnicodeString(Box<UnicodeString>),
    ColumnPosition(Box<ColumnPosition>),
    ColumnDef(Box<ColumnDef>),
    AlterColumn(Box<AlterColumn>),
    AlterSortKey(Box<AlterSortKey>),
    AlterSet(Box<AlterSet>),
    RenameColumn(Box<RenameColumn>),
    Comprehension(Box<Comprehension>),
    MergeTreeTTLAction(Box<MergeTreeTTLAction>),
    MergeTreeTTL(Box<MergeTreeTTL>),
    IndexConstraintOption(Box<IndexConstraintOption>),
    ColumnConstraint(Box<ColumnConstraint>),
    PeriodForSystemTimeConstraint(Box<PeriodForSystemTimeConstraint>),
    CaseSpecificColumnConstraint(Box<CaseSpecificColumnConstraint>),
    CharacterSetColumnConstraint(Box<CharacterSetColumnConstraint>),
    CheckColumnConstraint(Box<CheckColumnConstraint>),
    AssumeColumnConstraint(Box<AssumeColumnConstraint>),
    CompressColumnConstraint(Box<CompressColumnConstraint>),
    DateFormatColumnConstraint(Box<DateFormatColumnConstraint>),
    EphemeralColumnConstraint(Box<EphemeralColumnConstraint>),
    WithOperator(Box<WithOperator>),
    GeneratedAsIdentityColumnConstraint(Box<GeneratedAsIdentityColumnConstraint>),
    AutoIncrementColumnConstraint(AutoIncrementColumnConstraint),
    CommentColumnConstraint(CommentColumnConstraint),
    GeneratedAsRowColumnConstraint(Box<GeneratedAsRowColumnConstraint>),
    IndexColumnConstraint(Box<IndexColumnConstraint>),
    MaskingPolicyColumnConstraint(Box<MaskingPolicyColumnConstraint>),
    NotNullColumnConstraint(Box<NotNullColumnConstraint>),
    PrimaryKeyColumnConstraint(Box<PrimaryKeyColumnConstraint>),
    UniqueColumnConstraint(Box<UniqueColumnConstraint>),
    WatermarkColumnConstraint(Box<WatermarkColumnConstraint>),
    ComputedColumnConstraint(Box<ComputedColumnConstraint>),
    InOutColumnConstraint(Box<InOutColumnConstraint>),
    DefaultColumnConstraint(Box<DefaultColumnConstraint>),
    PathColumnConstraint(Box<PathColumnConstraint>),
    Constraint(Box<Constraint>),
    Export(Box<Export>),
    Filter(Box<Filter>),
    Changes(Box<Changes>),
    CopyParameter(Box<CopyParameter>),
    Credentials(Box<Credentials>),
    Directory(Box<Directory>),
    ForeignKey(Box<ForeignKey>),
    ColumnPrefix(Box<ColumnPrefix>),
    PrimaryKey(Box<PrimaryKey>),
    IntoClause(Box<IntoClause>),
    JoinHint(Box<JoinHint>),
    Opclass(Box<Opclass>),
    Index(Box<Index>),
    IndexParameters(Box<IndexParameters>),
    ConditionalInsert(Box<ConditionalInsert>),
    MultitableInserts(Box<MultitableInserts>),
    OnConflict(Box<OnConflict>),
    OnCondition(Box<OnCondition>),
    Returning(Box<Returning>),
    Introducer(Box<Introducer>),
    PartitionRange(Box<PartitionRange>),
    Fetch(Box<Fetch>),
    Group(Box<Group>),
    Cube(Box<Cube>),
    Rollup(Box<Rollup>),
    GroupingSets(Box<GroupingSets>),
    LimitOptions(Box<LimitOptions>),
    Lateral(Box<Lateral>),
    TableFromRows(Box<TableFromRows>),
    RowsFrom(Box<RowsFrom>),
    MatchRecognizeMeasure(Box<MatchRecognizeMeasure>),
    WithFill(Box<WithFill>),
    Property(Box<Property>),
    GrantPrivilege(Box<GrantPrivilege>),
    GrantPrincipal(Box<GrantPrincipal>),
    AllowedValuesProperty(Box<AllowedValuesProperty>),
    AlgorithmProperty(Box<AlgorithmProperty>),
    AutoIncrementProperty(Box<AutoIncrementProperty>),
    AutoRefreshProperty(Box<AutoRefreshProperty>),
    BackupProperty(Box<BackupProperty>),
    BuildProperty(Box<BuildProperty>),
    BlockCompressionProperty(Box<BlockCompressionProperty>),
    CharacterSetProperty(Box<CharacterSetProperty>),
    ChecksumProperty(Box<ChecksumProperty>),
    CollateProperty(Box<CollateProperty>),
    DataBlocksizeProperty(Box<DataBlocksizeProperty>),
    DataDeletionProperty(Box<DataDeletionProperty>),
    DefinerProperty(Box<DefinerProperty>),
    DistKeyProperty(Box<DistKeyProperty>),
    DistributedByProperty(Box<DistributedByProperty>),
    DistStyleProperty(Box<DistStyleProperty>),
    DuplicateKeyProperty(Box<DuplicateKeyProperty>),
    EngineProperty(Box<EngineProperty>),
    ToTableProperty(Box<ToTableProperty>),
    ExecuteAsProperty(Box<ExecuteAsProperty>),
    ExternalProperty(Box<ExternalProperty>),
    FallbackProperty(Box<FallbackProperty>),
    FileFormatProperty(Box<FileFormatProperty>),
    CredentialsProperty(Box<CredentialsProperty>),
    FreespaceProperty(Box<FreespaceProperty>),
    InheritsProperty(Box<InheritsProperty>),
    InputModelProperty(Box<InputModelProperty>),
    OutputModelProperty(Box<OutputModelProperty>),
    IsolatedLoadingProperty(Box<IsolatedLoadingProperty>),
    JournalProperty(Box<JournalProperty>),
    LanguageProperty(Box<LanguageProperty>),
    EnviromentProperty(Box<EnviromentProperty>),
    ClusteredByProperty(Box<ClusteredByProperty>),
    DictProperty(Box<DictProperty>),
    DictRange(Box<DictRange>),
    OnCluster(Box<OnCluster>),
    LikeProperty(Box<LikeProperty>),
    LocationProperty(Box<LocationProperty>),
    LockProperty(Box<LockProperty>),
    LockingProperty(Box<LockingProperty>),
    LogProperty(Box<LogProperty>),
    MaterializedProperty(Box<MaterializedProperty>),
    MergeBlockRatioProperty(Box<MergeBlockRatioProperty>),
    OnProperty(Box<OnProperty>),
    OnCommitProperty(Box<OnCommitProperty>),
    PartitionedByProperty(Box<PartitionedByProperty>),
    PartitionByProperty(Box<PartitionByProperty>),
    PartitionedByBucket(Box<PartitionedByBucket>),
    ClusterByColumnsProperty(Box<ClusterByColumnsProperty>),
    PartitionByTruncate(Box<PartitionByTruncate>),
    PartitionByRangeProperty(Box<PartitionByRangeProperty>),
    PartitionByRangePropertyDynamic(Box<PartitionByRangePropertyDynamic>),
    PartitionByListProperty(Box<PartitionByListProperty>),
    PartitionList(Box<PartitionList>),
    Partition(Box<Partition>),
    RefreshTriggerProperty(Box<RefreshTriggerProperty>),
    UniqueKeyProperty(Box<UniqueKeyProperty>),
    RollupProperty(Box<RollupProperty>),
    PartitionBoundSpec(Box<PartitionBoundSpec>),
    PartitionedOfProperty(Box<PartitionedOfProperty>),
    RemoteWithConnectionModelProperty(Box<RemoteWithConnectionModelProperty>),
    ReturnsProperty(Box<ReturnsProperty>),
    RowFormatProperty(Box<RowFormatProperty>),
    RowFormatDelimitedProperty(Box<RowFormatDelimitedProperty>),
    RowFormatSerdeProperty(Box<RowFormatSerdeProperty>),
    QueryTransform(Box<QueryTransform>),
    SampleProperty(Box<SampleProperty>),
    SecurityProperty(Box<SecurityProperty>),
    SchemaCommentProperty(Box<SchemaCommentProperty>),
    SemanticView(Box<SemanticView>),
    SerdeProperties(Box<SerdeProperties>),
    SetProperty(Box<SetProperty>),
    SharingProperty(Box<SharingProperty>),
    SetConfigProperty(Box<SetConfigProperty>),
    SettingsProperty(Box<SettingsProperty>),
    SortKeyProperty(Box<SortKeyProperty>),
    SqlReadWriteProperty(Box<SqlReadWriteProperty>),
    SqlSecurityProperty(Box<SqlSecurityProperty>),
    StabilityProperty(Box<StabilityProperty>),
    StorageHandlerProperty(Box<StorageHandlerProperty>),
    TemporaryProperty(Box<TemporaryProperty>),
    Tags(Box<Tags>),
    TransformModelProperty(Box<TransformModelProperty>),
    TransientProperty(Box<TransientProperty>),
    UsingTemplateProperty(Box<UsingTemplateProperty>),
    ViewAttributeProperty(Box<ViewAttributeProperty>),
    VolatileProperty(Box<VolatileProperty>),
    WithDataProperty(Box<WithDataProperty>),
    WithJournalTableProperty(Box<WithJournalTableProperty>),
    WithSchemaBindingProperty(Box<WithSchemaBindingProperty>),
    WithSystemVersioningProperty(Box<WithSystemVersioningProperty>),
    WithProcedureOptions(Box<WithProcedureOptions>),
    EncodeProperty(Box<EncodeProperty>),
    IncludeProperty(Box<IncludeProperty>),
    Properties(Box<Properties>),
    OptionsProperty(Box<OptionsProperty>),
    InputOutputFormat(Box<InputOutputFormat>),
    Reference(Box<Reference>),
    QueryOption(Box<QueryOption>),
    WithTableHint(Box<WithTableHint>),
    IndexTableHint(Box<IndexTableHint>),
    HistoricalData(Box<HistoricalData>),
    Get(Box<Get>),
    SetOperation(Box<SetOperation>),
    Var(Box<Var>),
    Variadic(Box<Variadic>),
    Version(Box<Version>),
    Schema(Box<Schema>),
    Lock(Box<Lock>),
    TableSample(Box<TableSample>),
    Tag(Box<Tag>),
    UnpivotColumns(Box<UnpivotColumns>),
    WindowSpec(Box<WindowSpec>),
    SessionParameter(Box<SessionParameter>),
    PseudoType(Box<PseudoType>),
    ObjectIdentifier(Box<ObjectIdentifier>),
    Transaction(Box<Transaction>),
    Commit(Box<Commit>),
    Rollback(Box<Rollback>),
    AlterSession(Box<AlterSession>),
    Analyze(Box<Analyze>),
    AnalyzeStatistics(Box<AnalyzeStatistics>),
    AnalyzeHistogram(Box<AnalyzeHistogram>),
    AnalyzeSample(Box<AnalyzeSample>),
    AnalyzeListChainedRows(Box<AnalyzeListChainedRows>),
    AnalyzeDelete(Box<AnalyzeDelete>),
    AnalyzeWith(Box<AnalyzeWith>),
    AnalyzeValidate(Box<AnalyzeValidate>),
    AddPartition(Box<AddPartition>),
    AttachOption(Box<AttachOption>),
    DropPartition(Box<DropPartition>),
    ReplacePartition(Box<ReplacePartition>),
    DPipe(Box<DPipe>),
    Operator(Box<Operator>),
    PivotAny(Box<PivotAny>),
    Aliases(Box<Aliases>),
    AtIndex(Box<AtIndex>),
    FromTimeZone(Box<FromTimeZone>),
    FormatPhrase(Box<FormatPhrase>),
    ForIn(Box<ForIn>),
    TimeUnit(Box<TimeUnit>),
    IntervalOp(Box<IntervalOp>),
    IntervalSpan(Box<IntervalSpan>),
    HavingMax(Box<HavingMax>),
    CosineDistance(Box<CosineDistance>),
    DotProduct(Box<DotProduct>),
    EuclideanDistance(Box<EuclideanDistance>),
    ManhattanDistance(Box<ManhattanDistance>),
    JarowinklerSimilarity(Box<JarowinklerSimilarity>),
    Booland(Box<Booland>),
    Boolor(Box<Boolor>),
    ParameterizedAgg(Box<ParameterizedAgg>),
    ArgMax(Box<ArgMax>),
    ArgMin(Box<ArgMin>),
    ApproxTopK(Box<ApproxTopK>),
    ApproxTopKAccumulate(Box<ApproxTopKAccumulate>),
    ApproxTopKCombine(Box<ApproxTopKCombine>),
    ApproxTopKEstimate(Box<ApproxTopKEstimate>),
    ApproxTopSum(Box<ApproxTopSum>),
    ApproxQuantiles(Box<ApproxQuantiles>),
    Minhash(Box<Minhash>),
    FarmFingerprint(Box<FarmFingerprint>),
    Float64(Box<Float64>),
    Transform(Box<Transform>),
    Translate(Box<Translate>),
    Grouping(Box<Grouping>),
    GroupingId(Box<GroupingId>),
    Anonymous(Box<Anonymous>),
    AnonymousAggFunc(Box<AnonymousAggFunc>),
    CombinedAggFunc(Box<CombinedAggFunc>),
    CombinedParameterizedAgg(Box<CombinedParameterizedAgg>),
    HashAgg(Box<HashAgg>),
    Hll(Box<Hll>),
    Apply(Box<Apply>),
    ToBoolean(Box<ToBoolean>),
    List(Box<List>),
    ToMap(Box<ToMap>),
    Pad(Box<Pad>),
    ToChar(Box<ToChar>),
    ToNumber(Box<ToNumber>),
    ToDouble(Box<ToDouble>),
    Int64(Box<UnaryFunc>),
    StringFunc(Box<StringFunc>),
    ToDecfloat(Box<ToDecfloat>),
    TryToDecfloat(Box<TryToDecfloat>),
    ToFile(Box<ToFile>),
    Columns(Box<Columns>),
    ConvertToCharset(Box<ConvertToCharset>),
    ConvertTimezone(Box<ConvertTimezone>),
    GenerateSeries(Box<GenerateSeries>),
    AIAgg(Box<AIAgg>),
    AIClassify(Box<AIClassify>),
    ArrayAll(Box<ArrayAll>),
    ArrayAny(Box<ArrayAny>),
    ArrayConstructCompact(Box<ArrayConstructCompact>),
    StPoint(Box<StPoint>),
    StDistance(Box<StDistance>),
    StringToArray(Box<StringToArray>),
    ArraySum(Box<ArraySum>),
    ObjectAgg(Box<ObjectAgg>),
    CastToStrType(Box<CastToStrType>),
    CheckJson(Box<CheckJson>),
    CheckXml(Box<CheckXml>),
    TranslateCharacters(Box<TranslateCharacters>),
    CurrentSchemas(Box<CurrentSchemas>),
    CurrentDatetime(Box<CurrentDatetime>),
    Localtime(Box<Localtime>),
    Localtimestamp(Box<Localtimestamp>),
    Systimestamp(Box<Systimestamp>),
    CurrentSchema(Box<CurrentSchema>),
    CurrentUser(Box<CurrentUser>),
    UtcTime(Box<UtcTime>),
    UtcTimestamp(Box<UtcTimestamp>),
    Timestamp(Box<TimestampFunc>),
    DateBin(Box<DateBin>),
    Datetime(Box<Datetime>),
    DatetimeAdd(Box<DatetimeAdd>),
    DatetimeSub(Box<DatetimeSub>),
    DatetimeDiff(Box<DatetimeDiff>),
    DatetimeTrunc(Box<DatetimeTrunc>),
    Dayname(Box<Dayname>),
    MakeInterval(Box<MakeInterval>),
    PreviousDay(Box<PreviousDay>),
    Elt(Box<Elt>),
    TimestampAdd(Box<TimestampAdd>),
    TimestampSub(Box<TimestampSub>),
    TimestampDiff(Box<TimestampDiff>),
    TimeSlice(Box<TimeSlice>),
    TimeAdd(Box<TimeAdd>),
    TimeSub(Box<TimeSub>),
    TimeDiff(Box<TimeDiff>),
    TimeTrunc(Box<TimeTrunc>),
    DateFromParts(Box<DateFromParts>),
    TimeFromParts(Box<TimeFromParts>),
    DecodeCase(Box<DecodeCase>),
    Decrypt(Box<Decrypt>),
    DecryptRaw(Box<DecryptRaw>),
    Encode(Box<Encode>),
    Encrypt(Box<Encrypt>),
    EncryptRaw(Box<EncryptRaw>),
    EqualNull(Box<EqualNull>),
    ToBinary(Box<ToBinary>),
    Base64DecodeBinary(Box<Base64DecodeBinary>),
    Base64DecodeString(Box<Base64DecodeString>),
    Base64Encode(Box<Base64Encode>),
    TryBase64DecodeBinary(Box<TryBase64DecodeBinary>),
    TryBase64DecodeString(Box<TryBase64DecodeString>),
    GapFill(Box<GapFill>),
    GenerateDateArray(Box<GenerateDateArray>),
    GenerateTimestampArray(Box<GenerateTimestampArray>),
    GetExtract(Box<GetExtract>),
    Getbit(Box<Getbit>),
    OverflowTruncateBehavior(Box<OverflowTruncateBehavior>),
    HexEncode(Box<HexEncode>),
    Compress(Box<Compress>),
    DecompressBinary(Box<DecompressBinary>),
    DecompressString(Box<DecompressString>),
    Xor(Box<Xor>),
    Nullif(Box<Nullif>),
    JSON(Box<JSON>),
    JSONPath(Box<JSONPath>),
    JSONPathFilter(Box<JSONPathFilter>),
    JSONPathKey(Box<JSONPathKey>),
    JSONPathRecursive(Box<JSONPathRecursive>),
    JSONPathScript(Box<JSONPathScript>),
    JSONPathSlice(Box<JSONPathSlice>),
    JSONPathSelector(Box<JSONPathSelector>),
    JSONPathSubscript(Box<JSONPathSubscript>),
    JSONPathUnion(Box<JSONPathUnion>),
    Format(Box<Format>),
    JSONKeys(Box<JSONKeys>),
    JSONKeyValue(Box<JSONKeyValue>),
    JSONKeysAtDepth(Box<JSONKeysAtDepth>),
    JSONObject(Box<JSONObject>),
    JSONObjectAgg(Box<JSONObjectAgg>),
    JSONBObjectAgg(Box<JSONBObjectAgg>),
    JSONArray(Box<JSONArray>),
    JSONArrayAgg(Box<JSONArrayAgg>),
    JSONExists(Box<JSONExists>),
    JSONColumnDef(Box<JSONColumnDef>),
    JSONSchema(Box<JSONSchema>),
    JSONSet(Box<JSONSet>),
    JSONStripNulls(Box<JSONStripNulls>),
    JSONValue(Box<JSONValue>),
    JSONValueArray(Box<JSONValueArray>),
    JSONRemove(Box<JSONRemove>),
    JSONTable(Box<JSONTable>),
    JSONType(Box<JSONType>),
    ObjectInsert(Box<ObjectInsert>),
    OpenJSONColumnDef(Box<OpenJSONColumnDef>),
    OpenJSON(Box<OpenJSON>),
    JSONBExists(Box<JSONBExists>),
    JSONBContains(Box<BinaryFunc>),
    JSONBExtract(Box<BinaryFunc>),
    JSONCast(Box<JSONCast>),
    JSONExtract(Box<JSONExtract>),
    JSONExtractQuote(Box<JSONExtractQuote>),
    JSONExtractArray(Box<JSONExtractArray>),
    JSONExtractScalar(Box<JSONExtractScalar>),
    JSONBExtractScalar(Box<JSONBExtractScalar>),
    JSONFormat(Box<JSONFormat>),
    JSONBool(Box<UnaryFunc>),
    JSONPathRoot(JSONPathRoot),
    JSONArrayAppend(Box<JSONArrayAppend>),
    JSONArrayContains(Box<JSONArrayContains>),
    JSONArrayInsert(Box<JSONArrayInsert>),
    ParseJSON(Box<ParseJSON>),
    ParseUrl(Box<ParseUrl>),
    ParseIp(Box<ParseIp>),
    ParseTime(Box<ParseTime>),
    ParseDatetime(Box<ParseDatetime>),
    Map(Box<Map>),
    MapCat(Box<MapCat>),
    MapDelete(Box<MapDelete>),
    MapInsert(Box<MapInsert>),
    MapPick(Box<MapPick>),
    ScopeResolution(Box<ScopeResolution>),
    Slice(Box<Slice>),
    VarMap(Box<VarMap>),
    MatchAgainst(Box<MatchAgainst>),
    MD5Digest(Box<MD5Digest>),
    MD5NumberLower64(Box<UnaryFunc>),
    MD5NumberUpper64(Box<UnaryFunc>),
    Monthname(Box<Monthname>),
    Ntile(Box<Ntile>),
    Normalize(Box<Normalize>),
    Normal(Box<Normal>),
    Predict(Box<Predict>),
    MLTranslate(Box<MLTranslate>),
    FeaturesAtTime(Box<FeaturesAtTime>),
    GenerateEmbedding(Box<GenerateEmbedding>),
    MLForecast(Box<MLForecast>),
    ModelAttribute(Box<ModelAttribute>),
    VectorSearch(Box<VectorSearch>),
    Quantile(Box<Quantile>),
    ApproxQuantile(Box<ApproxQuantile>),
    ApproxPercentileEstimate(Box<ApproxPercentileEstimate>),
    Randn(Box<Randn>),
    Randstr(Box<Randstr>),
    RangeN(Box<RangeN>),
    RangeBucket(Box<RangeBucket>),
    ReadCSV(Box<ReadCSV>),
    ReadParquet(Box<ReadParquet>),
    Reduce(Box<Reduce>),
    RegexpExtractAll(Box<RegexpExtractAll>),
    RegexpILike(Box<RegexpILike>),
    RegexpFullMatch(Box<RegexpFullMatch>),
    RegexpInstr(Box<RegexpInstr>),
    RegexpSplit(Box<RegexpSplit>),
    RegexpCount(Box<RegexpCount>),
    RegrValx(Box<RegrValx>),
    RegrValy(Box<RegrValy>),
    RegrAvgy(Box<RegrAvgy>),
    RegrAvgx(Box<RegrAvgx>),
    RegrCount(Box<RegrCount>),
    RegrIntercept(Box<RegrIntercept>),
    RegrR2(Box<RegrR2>),
    RegrSxx(Box<RegrSxx>),
    RegrSxy(Box<RegrSxy>),
    RegrSyy(Box<RegrSyy>),
    RegrSlope(Box<RegrSlope>),
    SafeAdd(Box<SafeAdd>),
    SafeDivide(Box<SafeDivide>),
    SafeMultiply(Box<SafeMultiply>),
    SafeSubtract(Box<SafeSubtract>),
    SHA2(Box<SHA2>),
    SHA2Digest(Box<SHA2Digest>),
    SortArray(Box<SortArray>),
    SplitPart(Box<SplitPart>),
    SubstringIndex(Box<SubstringIndex>),
    StandardHash(Box<StandardHash>),
    StrPosition(Box<StrPosition>),
    Search(Box<Search>),
    SearchIp(Box<SearchIp>),
    StrToDate(Box<StrToDate>),
    DateStrToDate(Box<UnaryFunc>),
    DateToDateStr(Box<UnaryFunc>),
    StrToTime(Box<StrToTime>),
    StrToUnix(Box<StrToUnix>),
    StrToMap(Box<StrToMap>),
    NumberToStr(Box<NumberToStr>),
    FromBase(Box<FromBase>),
    Stuff(Box<Stuff>),
    TimeToStr(Box<TimeToStr>),
    TimeStrToTime(Box<TimeStrToTime>),
    TsOrDsAdd(Box<TsOrDsAdd>),
    TsOrDsDiff(Box<TsOrDsDiff>),
    TsOrDsToDate(Box<TsOrDsToDate>),
    TsOrDsToTime(Box<TsOrDsToTime>),
    Unhex(Box<Unhex>),
    Uniform(Box<Uniform>),
    UnixToStr(Box<UnixToStr>),
    UnixToTime(Box<UnixToTime>),
    Uuid(Box<Uuid>),
    TimestampFromParts(Box<TimestampFromParts>),
    TimestampTzFromParts(Box<TimestampTzFromParts>),
    Corr(Box<Corr>),
    WidthBucket(Box<WidthBucket>),
    CovarSamp(Box<CovarSamp>),
    CovarPop(Box<CovarPop>),
    Week(Box<Week>),
    XMLElement(Box<XMLElement>),
    XMLGet(Box<XMLGet>),
    XMLTable(Box<XMLTable>),
    XMLKeyValueOption(Box<XMLKeyValueOption>),
    Zipf(Box<Zipf>),
    Merge(Box<Merge>),
    When(Box<When>),
    Whens(Box<Whens>),
    NextValueFor(Box<NextValueFor>),
    /// RETURN statement (DuckDB stored procedures)
    ReturnStmt(Box<Expression>),
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expression::Literal(lit) => write!(f, "{}", lit),
            Expression::Identifier(id) => write!(f, "{}", id),
            Expression::Column(col) => write!(f, "{}", col),
            Expression::Star(_) => write!(f, "*"),
            Expression::Null(_) => write!(f, "NULL"),
            Expression::Boolean(b) => write!(f, "{}", if b.value { "TRUE" } else { "FALSE" }),
            Expression::Select(_) => write!(f, "SELECT ..."),
            _ => write!(f, "{:?}", self),
        }
    }
}
