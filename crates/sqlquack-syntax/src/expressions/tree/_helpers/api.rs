//! Expressions: api.

use crate::expressions::tree::models::Expression;
use crate::tokens::models::Span;
use crate::traversal::types::AstNode;

impl Expression {
    /// The node's own source range: a located parse's, or the one the parser keeps on names.
    pub fn source_span(&self) -> Option<Span> {
        self.recorded_span()
    }

    /// The snake_case variant name serde uses, without serializing.
    pub fn variant_name(&self) -> &'static str {
        match self {
            Expression::Vertica(_) => "vertica",
            Expression::Literal(_) => "literal",
            Expression::Boolean(_) => "boolean",
            Expression::Null(_) => "null",
            Expression::Identifier(_) => "identifier",
            Expression::Column(_) => "column",
            Expression::Table(_) => "table",
            Expression::Star(_) => "star",
            Expression::BracedWildcard(_) => "braced_wildcard",
            Expression::Select(_) => "select",
            Expression::Union(_) => "union",
            Expression::Intersect(_) => "intersect",
            Expression::Except(_) => "except",
            Expression::Subquery(_) => "subquery",
            Expression::PipeOperator(_) => "pipe_operator",
            Expression::Pivot(_) => "pivot",
            Expression::PivotAlias(_) => "pivot_alias",
            Expression::Unpivot(_) => "unpivot",
            Expression::Values(_) => "values",
            Expression::PreWhere(_) => "pre_where",
            Expression::Stream(_) => "stream",
            Expression::UsingData(_) => "using_data",
            Expression::XmlNamespace(_) => "xml_namespace",
            Expression::Insert(_) => "insert",
            Expression::Update(_) => "update",
            Expression::Delete(_) => "delete",
            Expression::Copy(_) => "copy",
            Expression::Put(_) => "put",
            Expression::StageReference(_) => "stage_reference",
            Expression::Alias(_) => "alias",
            Expression::Cast(_) => "cast",
            Expression::Collation(_) => "collation",
            Expression::Case(_) => "case",
            Expression::And(_) => "and",
            Expression::Or(_) => "or",
            Expression::Add(_) => "add",
            Expression::Sub(_) => "sub",
            Expression::Mul(_) => "mul",
            Expression::Div(_) => "div",
            Expression::Mod(_) => "mod",
            Expression::Eq(_) => "eq",
            Expression::Neq(_) => "neq",
            Expression::Lt(_) => "lt",
            Expression::Lte(_) => "lte",
            Expression::Gt(_) => "gt",
            Expression::Gte(_) => "gte",
            Expression::Like(_) => "like",
            Expression::ILike(_) => "i_like",
            Expression::Match(_) => "match",
            Expression::BitwiseAnd(_) => "bitwise_and",
            Expression::BitwiseOr(_) => "bitwise_or",
            Expression::BitwiseXor(_) => "bitwise_xor",
            Expression::Concat(_) => "concat",
            Expression::Adjacent(_) => "adjacent",
            Expression::TsMatch(_) => "ts_match",
            Expression::PropertyEQ(_) => "property_e_q",
            Expression::ArrayContainsAll(_) => "array_contains_all",
            Expression::ArrayContainedBy(_) => "array_contained_by",
            Expression::ArrayOverlaps(_) => "array_overlaps",
            Expression::JSONBContainsAllTopKeys(_) => "j_s_o_n_b_contains_all_top_keys",
            Expression::JSONBContainsAnyTopKeys(_) => "j_s_o_n_b_contains_any_top_keys",
            Expression::JSONBDeleteAtPath(_) => "j_s_o_n_b_delete_at_path",
            Expression::ExtendsLeft(_) => "extends_left",
            Expression::ExtendsRight(_) => "extends_right",
            Expression::Not(_) => "not",
            Expression::Neg(_) => "neg",
            Expression::Pos(_) => "pos",
            Expression::BitwiseNot(_) => "bitwise_not",
            Expression::In(_) => "in",
            Expression::Between(_) => "between",
            Expression::IsNull(_) => "is_null",
            Expression::IsTrue(_) => "is_true",
            Expression::IsFalse(_) => "is_false",
            Expression::IsJson(_) => "is_json",
            Expression::Is(_) => "is",
            Expression::Exists(_) => "exists",
            Expression::MemberOf(_) => "member_of",
            Expression::Function(_) => "function",
            Expression::Upsert(_) => "upsert",
            Expression::StorageProperty(_) => "storage_property",
            Expression::Hierarchy(_) => "hierarchy",
            Expression::ViewParameter(_) => "view_parameter",
            Expression::Call(_) => "call",
            Expression::AggregateFunction(_) => "aggregate_function",
            Expression::WindowFunction(_) => "window_function",
            Expression::From(_) => "from",
            Expression::Join(_) => "join",
            Expression::JoinedTable(_) => "joined_table",
            Expression::Where(_) => "where",
            Expression::GroupBy(_) => "group_by",
            Expression::Having(_) => "having",
            Expression::OrderBy(_) => "order_by",
            Expression::Limit(_) => "limit",
            Expression::Offset(_) => "offset",
            Expression::Qualify(_) => "qualify",
            Expression::With(_) => "with",
            Expression::Cte(_) => "cte",
            Expression::DistributeBy(_) => "distribute_by",
            Expression::ClusterBy(_) => "cluster_by",
            Expression::SortBy(_) => "sort_by",
            Expression::LateralView(_) => "lateral_view",
            Expression::Hint(_) => "hint",
            Expression::Pseudocolumn(_) => "pseudocolumn",
            Expression::Connect(_) => "connect",
            Expression::Prior(_) => "prior",
            Expression::ConnectByRoot(_) => "connect_by_root",
            Expression::MatchRecognize(_) => "match_recognize",
            Expression::Resample(_) => "resample",
            Expression::Ordered(_) => "ordered",
            Expression::Window(_) => "window",
            Expression::Over(_) => "over",
            Expression::WithinGroup(_) => "within_group",
            Expression::DataType(_) => "data_type",
            Expression::Array(_) => "array",
            Expression::Struct(_) => "struct",
            Expression::Tuple(_) => "tuple",
            Expression::Interval(_) => "interval",
            Expression::ConcatWs(_) => "concat_ws",
            Expression::Substring(_) => "substring",
            Expression::Upper(_) => "upper",
            Expression::Lower(_) => "lower",
            Expression::Length(_) => "length",
            Expression::Trim(_) => "trim",
            Expression::LTrim(_) => "l_trim",
            Expression::RTrim(_) => "r_trim",
            Expression::Replace(_) => "replace",
            Expression::Reverse(_) => "reverse",
            Expression::Left(_) => "left",
            Expression::Right(_) => "right",
            Expression::Repeat(_) => "repeat",
            Expression::Lpad(_) => "lpad",
            Expression::Rpad(_) => "rpad",
            Expression::Split(_) => "split",
            Expression::RegexpLike(_) => "regexp_like",
            Expression::RegexpReplace(_) => "regexp_replace",
            Expression::RegexpExtract(_) => "regexp_extract",
            Expression::Overlay(_) => "overlay",
            Expression::Abs(_) => "abs",
            Expression::Round(_) => "round",
            Expression::Floor(_) => "floor",
            Expression::Ceil(_) => "ceil",
            Expression::Power(_) => "power",
            Expression::Sqrt(_) => "sqrt",
            Expression::Cbrt(_) => "cbrt",
            Expression::Ln(_) => "ln",
            Expression::Log(_) => "log",
            Expression::Exp(_) => "exp",
            Expression::Sign(_) => "sign",
            Expression::Greatest(_) => "greatest",
            Expression::Least(_) => "least",
            Expression::CurrentDate(_) => "current_date",
            Expression::CurrentTime(_) => "current_time",
            Expression::CurrentTimestamp(_) => "current_timestamp",
            Expression::CurrentTimestampLTZ(_) => "current_timestamp_l_t_z",
            Expression::AtTimeZone(_) => "at_time_zone",
            Expression::DateAdd(_) => "date_add",
            Expression::DateSub(_) => "date_sub",
            Expression::DateDiff(_) => "date_diff",
            Expression::DateTrunc(_) => "date_trunc",
            Expression::Extract(_) => "extract",
            Expression::ToDate(_) => "to_date",
            Expression::ToTimestamp(_) => "to_timestamp",
            Expression::Date(_) => "date",
            Expression::Time(_) => "time",
            Expression::DateFromUnixDate(_) => "date_from_unix_date",
            Expression::UnixDate(_) => "unix_date",
            Expression::UnixSeconds(_) => "unix_seconds",
            Expression::UnixMillis(_) => "unix_millis",
            Expression::UnixMicros(_) => "unix_micros",
            Expression::UnixToTimeStr(_) => "unix_to_time_str",
            Expression::TimeStrToDate(_) => "time_str_to_date",
            Expression::DateToDi(_) => "date_to_di",
            Expression::DiToDate(_) => "di_to_date",
            Expression::TsOrDiToDi(_) => "ts_or_di_to_di",
            Expression::TsOrDsToDatetime(_) => "ts_or_ds_to_datetime",
            Expression::TsOrDsToTimestamp(_) => "ts_or_ds_to_timestamp",
            Expression::YearOfWeek(_) => "year_of_week",
            Expression::YearOfWeekIso(_) => "year_of_week_iso",
            Expression::Coalesce(_) => "coalesce",
            Expression::NullIf(_) => "null_if",
            Expression::IfFunc(_) => "if_func",
            Expression::IfNull(_) => "if_null",
            Expression::Nvl(_) => "nvl",
            Expression::Nvl2(_) => "nvl2",
            Expression::TryCast(_) => "try_cast",
            Expression::SafeCast(_) => "safe_cast",
            Expression::Count(_) => "count",
            Expression::Sum(_) => "sum",
            Expression::Avg(_) => "avg",
            Expression::Min(_) => "min",
            Expression::Max(_) => "max",
            Expression::GroupConcat(_) => "group_concat",
            Expression::StringAgg(_) => "string_agg",
            Expression::ListAgg(_) => "list_agg",
            Expression::ArrayAgg(_) => "array_agg",
            Expression::CountIf(_) => "count_if",
            Expression::SumIf(_) => "sum_if",
            Expression::Stddev(_) => "stddev",
            Expression::StddevPop(_) => "stddev_pop",
            Expression::StddevSamp(_) => "stddev_samp",
            Expression::Variance(_) => "variance",
            Expression::VarPop(_) => "var_pop",
            Expression::VarSamp(_) => "var_samp",
            Expression::Median(_) => "median",
            Expression::Mode(_) => "mode",
            Expression::First(_) => "first",
            Expression::Last(_) => "last",
            Expression::AnyValue(_) => "any_value",
            Expression::ApproxDistinct(_) => "approx_distinct",
            Expression::ApproxCountDistinct(_) => "approx_count_distinct",
            Expression::ApproxPercentile(_) => "approx_percentile",
            Expression::Percentile(_) => "percentile",
            Expression::LogicalAnd(_) => "logical_and",
            Expression::LogicalOr(_) => "logical_or",
            Expression::Skewness(_) => "skewness",
            Expression::BitwiseCount(_) => "bitwise_count",
            Expression::ArrayConcatAgg(_) => "array_concat_agg",
            Expression::ArrayUniqueAgg(_) => "array_unique_agg",
            Expression::BoolXorAgg(_) => "bool_xor_agg",
            Expression::RowNumber(_) => "row_number",
            Expression::Rank(_) => "rank",
            Expression::DenseRank(_) => "dense_rank",
            Expression::NTile(_) => "n_tile",
            Expression::Lead(_) => "lead",
            Expression::Lag(_) => "lag",
            Expression::FirstValue(_) => "first_value",
            Expression::LastValue(_) => "last_value",
            Expression::NthValue(_) => "nth_value",
            Expression::PercentRank(_) => "percent_rank",
            Expression::CumeDist(_) => "cume_dist",
            Expression::PercentileCont(_) => "percentile_cont",
            Expression::PercentileDisc(_) => "percentile_disc",
            Expression::Contains(_) => "contains",
            Expression::StartsWith(_) => "starts_with",
            Expression::EndsWith(_) => "ends_with",
            Expression::Position(_) => "position",
            Expression::Initcap(_) => "initcap",
            Expression::Ascii(_) => "ascii",
            Expression::Chr(_) => "chr",
            Expression::CharFunc(_) => "char_func",
            Expression::Soundex(_) => "soundex",
            Expression::Levenshtein(_) => "levenshtein",
            Expression::ByteLength(_) => "byte_length",
            Expression::Hex(_) => "hex",
            Expression::LowerHex(_) => "lower_hex",
            Expression::Unicode(_) => "unicode",
            Expression::ModFunc(_) => "mod_func",
            Expression::Random(_) => "random",
            Expression::Rand(_) => "rand",
            Expression::TruncFunc(_) => "trunc_func",
            Expression::Pi(_) => "pi",
            Expression::Radians(_) => "radians",
            Expression::Degrees(_) => "degrees",
            Expression::Sin(_) => "sin",
            Expression::Cos(_) => "cos",
            Expression::Tan(_) => "tan",
            Expression::Asin(_) => "asin",
            Expression::Acos(_) => "acos",
            Expression::Atan(_) => "atan",
            Expression::Atan2(_) => "atan2",
            Expression::IsNan(_) => "is_nan",
            Expression::IsInf(_) => "is_inf",
            Expression::IntDiv(_) => "int_div",
            Expression::Decode(_) => "decode",
            Expression::DateFormat(_) => "date_format",
            Expression::FormatDate(_) => "format_date",
            Expression::Year(_) => "year",
            Expression::Month(_) => "month",
            Expression::Day(_) => "day",
            Expression::Hour(_) => "hour",
            Expression::Minute(_) => "minute",
            Expression::Second(_) => "second",
            Expression::DayOfWeek(_) => "day_of_week",
            Expression::DayOfWeekIso(_) => "day_of_week_iso",
            Expression::DayOfMonth(_) => "day_of_month",
            Expression::DayOfYear(_) => "day_of_year",
            Expression::WeekOfYear(_) => "week_of_year",
            Expression::Quarter(_) => "quarter",
            Expression::AddMonths(_) => "add_months",
            Expression::MonthsBetween(_) => "months_between",
            Expression::LastDay(_) => "last_day",
            Expression::NextDay(_) => "next_day",
            Expression::Epoch(_) => "epoch",
            Expression::EpochMs(_) => "epoch_ms",
            Expression::FromUnixtime(_) => "from_unixtime",
            Expression::UnixTimestamp(_) => "unix_timestamp",
            Expression::MakeDate(_) => "make_date",
            Expression::MakeTimestamp(_) => "make_timestamp",
            Expression::TimestampTrunc(_) => "timestamp_trunc",
            Expression::TimeStrToUnix(_) => "time_str_to_unix",
            Expression::SessionUser(_) => "session_user",
            Expression::SHA(_) => "s_h_a",
            Expression::SHA1Digest(_) => "s_h_a1_digest",
            Expression::TimeToUnix(_) => "time_to_unix",
            Expression::ArrayFunc(_) => "array_func",
            Expression::ArrayLength(_) => "array_length",
            Expression::ArraySize(_) => "array_size",
            Expression::Cardinality(_) => "cardinality",
            Expression::ArrayContains(_) => "array_contains",
            Expression::ArrayPosition(_) => "array_position",
            Expression::ArrayAppend(_) => "array_append",
            Expression::ArrayPrepend(_) => "array_prepend",
            Expression::ArrayConcat(_) => "array_concat",
            Expression::ArraySort(_) => "array_sort",
            Expression::ArrayReverse(_) => "array_reverse",
            Expression::ArrayDistinct(_) => "array_distinct",
            Expression::ArrayJoin(_) => "array_join",
            Expression::ArrayToString(_) => "array_to_string",
            Expression::Unnest(_) => "unnest",
            Expression::Explode(_) => "explode",
            Expression::ExplodeOuter(_) => "explode_outer",
            Expression::ArrayFilter(_) => "array_filter",
            Expression::ArrayTransform(_) => "array_transform",
            Expression::ArrayFlatten(_) => "array_flatten",
            Expression::ArrayCompact(_) => "array_compact",
            Expression::ArrayIntersect(_) => "array_intersect",
            Expression::ArrayUnion(_) => "array_union",
            Expression::ArrayExcept(_) => "array_except",
            Expression::ArrayRemove(_) => "array_remove",
            Expression::ArrayZip(_) => "array_zip",
            Expression::Sequence(_) => "sequence",
            Expression::Generate(_) => "generate",
            Expression::ExplodingGenerateSeries(_) => "exploding_generate_series",
            Expression::ToArray(_) => "to_array",
            Expression::StarMap(_) => "star_map",
            Expression::StructFunc(_) => "struct_func",
            Expression::StructExtract(_) => "struct_extract",
            Expression::NamedStruct(_) => "named_struct",
            Expression::MapFunc(_) => "map_func",
            Expression::MapFromEntries(_) => "map_from_entries",
            Expression::MapFromArrays(_) => "map_from_arrays",
            Expression::MapKeys(_) => "map_keys",
            Expression::MapValues(_) => "map_values",
            Expression::MapContainsKey(_) => "map_contains_key",
            Expression::MapConcat(_) => "map_concat",
            Expression::ElementAt(_) => "element_at",
            Expression::TransformKeys(_) => "transform_keys",
            Expression::TransformValues(_) => "transform_values",
            Expression::FunctionEmits(_) => "function_emits",
            Expression::JsonExtract(_) => "json_extract",
            Expression::JsonExtractScalar(_) => "json_extract_scalar",
            Expression::JsonExtractPath(_) => "json_extract_path",
            Expression::JsonArray(_) => "json_array",
            Expression::JsonObject(_) => "json_object",
            Expression::JsonQuery(_) => "json_query",
            Expression::JsonValue(_) => "json_value",
            Expression::JsonArrayLength(_) => "json_array_length",
            Expression::JsonKeys(_) => "json_keys",
            Expression::JsonType(_) => "json_type",
            Expression::ParseJson(_) => "parse_json",
            Expression::ToJson(_) => "to_json",
            Expression::JsonSet(_) => "json_set",
            Expression::JsonInsert(_) => "json_insert",
            Expression::JsonRemove(_) => "json_remove",
            Expression::JsonMergePatch(_) => "json_merge_patch",
            Expression::JsonArrayAgg(_) => "json_array_agg",
            Expression::JsonObjectAgg(_) => "json_object_agg",
            Expression::Convert(_) => "convert",
            Expression::Typeof(_) => "typeof",
            Expression::Lambda(_) => "lambda",
            Expression::Parameter(_) => "parameter",
            Expression::Placeholder(_) => "placeholder",
            Expression::NamedArgument(_) => "named_argument",
            Expression::TableArgument(_) => "table_argument",
            Expression::SqlComment(_) => "sql_comment",
            Expression::NullSafeEq(_) => "null_safe_eq",
            Expression::NullSafeNeq(_) => "null_safe_neq",
            Expression::Glob(_) => "glob",
            Expression::SimilarTo(_) => "similar_to",
            Expression::Any(_) => "any",
            Expression::All(_) => "all",
            Expression::Overlaps(_) => "overlaps",
            Expression::BitwiseLeftShift(_) => "bitwise_left_shift",
            Expression::BitwiseRightShift(_) => "bitwise_right_shift",
            Expression::BitwiseAndAgg(_) => "bitwise_and_agg",
            Expression::BitwiseOrAgg(_) => "bitwise_or_agg",
            Expression::BitwiseXorAgg(_) => "bitwise_xor_agg",
            Expression::Subscript(_) => "subscript",
            Expression::Dot(_) => "dot",
            Expression::MethodCall(_) => "method_call",
            Expression::ArraySlice(_) => "array_slice",
            Expression::CreateTable(_) => "create_table",
            Expression::DropTable(_) => "drop_table",
            Expression::Undrop(_) => "undrop",
            Expression::AlterTable(_) => "alter_table",
            Expression::SplitTable(_) => "split_table",
            Expression::FlashbackTable(_) => "flashback_table",
            Expression::CreateIndex(_) => "create_index",
            Expression::DropIndex(_) => "drop_index",
            Expression::CreateView(_) => "create_view",
            Expression::DropView(_) => "drop_view",
            Expression::AlterView(_) => "alter_view",
            Expression::AlterIndex(_) => "alter_index",
            Expression::Truncate(_) => "truncate",
            Expression::Use(_) => "use",
            Expression::Cache(_) => "cache",
            Expression::Uncache(_) => "uncache",
            Expression::LoadData(_) => "load_data",
            Expression::Pragma(_) => "pragma",
            Expression::Grant(_) => "grant",
            Expression::Revoke(_) => "revoke",
            Expression::Comment(_) => "comment",
            Expression::SetStatement(_) => "set_statement",
            Expression::CreateSchema(_) => "create_schema",
            Expression::DropSchema(_) => "drop_schema",
            Expression::DropNamespace(_) => "drop_namespace",
            Expression::CreateDatabase(_) => "create_database",
            Expression::DropDatabase(_) => "drop_database",
            Expression::CreateFunction(_) => "create_function",
            Expression::DropFunction(_) => "drop_function",
            Expression::CreateProcedure(_) => "create_procedure",
            Expression::DropProcedure(_) => "drop_procedure",
            Expression::CreateSequence(_) => "create_sequence",
            Expression::CreateSynonym(_) => "create_synonym",
            Expression::DropSequence(_) => "drop_sequence",
            Expression::AlterSequence(_) => "alter_sequence",
            Expression::CreateTrigger(_) => "create_trigger",
            Expression::DropTrigger(_) => "drop_trigger",
            Expression::CreateType(_) => "create_type",
            Expression::DropType(_) => "drop_type",
            Expression::Describe(_) => "describe",
            Expression::Show(_) => "show",
            Expression::Command(_) => "command",
            Expression::ScriptingBlock(_) => "scripting_block",
            Expression::TryCatch(_) => "try_catch",
            Expression::Kill(_) => "kill",
            Expression::Prepare(_) => "prepare",
            Expression::Execute(_) => "execute",
            Expression::Raw(_) => "raw",
            Expression::CreateTask(_) => "create_task",
            Expression::Paren(_) => "paren",
            Expression::Annotated(_) => "annotated",
            Expression::Refresh(_) => "refresh",
            Expression::LockingStatement(_) => "locking_statement",
            Expression::SequenceProperties(_) => "sequence_properties",
            Expression::TruncateTable(_) => "truncate_table",
            Expression::Clone(_) => "clone",
            Expression::Attach(_) => "attach",
            Expression::Detach(_) => "detach",
            Expression::Install(_) => "install",
            Expression::Summarize(_) => "summarize",
            Expression::Declare(_) => "declare",
            Expression::DeclareItem(_) => "declare_item",
            Expression::Set(_) => "set",
            Expression::Heredoc(_) => "heredoc",
            Expression::SetItem(_) => "set_item",
            Expression::QueryBand(_) => "query_band",
            Expression::UserDefinedFunction(_) => "user_defined_function",
            Expression::RecursiveWithSearch(_) => "recursive_with_search",
            Expression::ProjectionDef(_) => "projection_def",
            Expression::TableAlias(_) => "table_alias",
            Expression::ByteString(_) => "byte_string",
            Expression::HexStringExpr(_) => "hex_string_expr",
            Expression::UnicodeString(_) => "unicode_string",
            Expression::ColumnPosition(_) => "column_position",
            Expression::ColumnDef(_) => "column_def",
            Expression::AlterColumn(_) => "alter_column",
            Expression::AlterSortKey(_) => "alter_sort_key",
            Expression::AlterSet(_) => "alter_set",
            Expression::RenameColumn(_) => "rename_column",
            Expression::Comprehension(_) => "comprehension",
            Expression::MergeTreeTTLAction(_) => "merge_tree_t_t_l_action",
            Expression::MergeTreeTTL(_) => "merge_tree_t_t_l",
            Expression::IndexConstraintOption(_) => "index_constraint_option",
            Expression::ColumnConstraint(_) => "column_constraint",
            Expression::PeriodForSystemTimeConstraint(_) => "period_for_system_time_constraint",
            Expression::CaseSpecificColumnConstraint(_) => "case_specific_column_constraint",
            Expression::CharacterSetColumnConstraint(_) => "character_set_column_constraint",
            Expression::CheckColumnConstraint(_) => "check_column_constraint",
            Expression::AssumeColumnConstraint(_) => "assume_column_constraint",
            Expression::CompressColumnConstraint(_) => "compress_column_constraint",
            Expression::DateFormatColumnConstraint(_) => "date_format_column_constraint",
            Expression::EphemeralColumnConstraint(_) => "ephemeral_column_constraint",
            Expression::WithOperator(_) => "with_operator",
            Expression::GeneratedAsIdentityColumnConstraint(_) => {
                "generated_as_identity_column_constraint"
            }
            Expression::AutoIncrementColumnConstraint(_) => "auto_increment_column_constraint",
            Expression::CommentColumnConstraint(_) => "comment_column_constraint",
            Expression::GeneratedAsRowColumnConstraint(_) => "generated_as_row_column_constraint",
            Expression::IndexColumnConstraint(_) => "index_column_constraint",
            Expression::MaskingPolicyColumnConstraint(_) => "masking_policy_column_constraint",
            Expression::NotNullColumnConstraint(_) => "not_null_column_constraint",
            Expression::PrimaryKeyColumnConstraint(_) => "primary_key_column_constraint",
            Expression::UniqueColumnConstraint(_) => "unique_column_constraint",
            Expression::WatermarkColumnConstraint(_) => "watermark_column_constraint",
            Expression::ComputedColumnConstraint(_) => "computed_column_constraint",
            Expression::InOutColumnConstraint(_) => "in_out_column_constraint",
            Expression::DefaultColumnConstraint(_) => "default_column_constraint",
            Expression::PathColumnConstraint(_) => "path_column_constraint",
            Expression::Constraint(_) => "constraint",
            Expression::Export(_) => "export",
            Expression::Filter(_) => "filter",
            Expression::Changes(_) => "changes",
            Expression::CopyParameter(_) => "copy_parameter",
            Expression::Credentials(_) => "credentials",
            Expression::Directory(_) => "directory",
            Expression::ForeignKey(_) => "foreign_key",
            Expression::ColumnPrefix(_) => "column_prefix",
            Expression::PrimaryKey(_) => "primary_key",
            Expression::IntoClause(_) => "into_clause",
            Expression::JoinHint(_) => "join_hint",
            Expression::Opclass(_) => "opclass",
            Expression::Index(_) => "index",
            Expression::IndexParameters(_) => "index_parameters",
            Expression::ConditionalInsert(_) => "conditional_insert",
            Expression::MultitableInserts(_) => "multitable_inserts",
            Expression::OnConflict(_) => "on_conflict",
            Expression::OnCondition(_) => "on_condition",
            Expression::Returning(_) => "returning",
            Expression::Introducer(_) => "introducer",
            Expression::PartitionRange(_) => "partition_range",
            Expression::Fetch(_) => "fetch",
            Expression::Group(_) => "group",
            Expression::Cube(_) => "cube",
            Expression::Rollup(_) => "rollup",
            Expression::GroupingSets(_) => "grouping_sets",
            Expression::LimitOptions(_) => "limit_options",
            Expression::Lateral(_) => "lateral",
            Expression::TableFromRows(_) => "table_from_rows",
            Expression::RowsFrom(_) => "rows_from",
            Expression::MatchRecognizeMeasure(_) => "match_recognize_measure",
            Expression::WithFill(_) => "with_fill",
            Expression::Property(_) => "property",
            Expression::GrantPrivilege(_) => "grant_privilege",
            Expression::GrantPrincipal(_) => "grant_principal",
            Expression::AllowedValuesProperty(_) => "allowed_values_property",
            Expression::AlgorithmProperty(_) => "algorithm_property",
            Expression::AutoIncrementProperty(_) => "auto_increment_property",
            Expression::AutoRefreshProperty(_) => "auto_refresh_property",
            Expression::BackupProperty(_) => "backup_property",
            Expression::BuildProperty(_) => "build_property",
            Expression::BlockCompressionProperty(_) => "block_compression_property",
            Expression::CharacterSetProperty(_) => "character_set_property",
            Expression::ChecksumProperty(_) => "checksum_property",
            Expression::CollateProperty(_) => "collate_property",
            Expression::DataBlocksizeProperty(_) => "data_blocksize_property",
            Expression::DataDeletionProperty(_) => "data_deletion_property",
            Expression::DefinerProperty(_) => "definer_property",
            Expression::DistKeyProperty(_) => "dist_key_property",
            Expression::DistributedByProperty(_) => "distributed_by_property",
            Expression::DistStyleProperty(_) => "dist_style_property",
            Expression::DuplicateKeyProperty(_) => "duplicate_key_property",
            Expression::EngineProperty(_) => "engine_property",
            Expression::ToTableProperty(_) => "to_table_property",
            Expression::ExecuteAsProperty(_) => "execute_as_property",
            Expression::ExternalProperty(_) => "external_property",
            Expression::FallbackProperty(_) => "fallback_property",
            Expression::FileFormatProperty(_) => "file_format_property",
            Expression::CredentialsProperty(_) => "credentials_property",
            Expression::FreespaceProperty(_) => "freespace_property",
            Expression::InheritsProperty(_) => "inherits_property",
            Expression::InputModelProperty(_) => "input_model_property",
            Expression::OutputModelProperty(_) => "output_model_property",
            Expression::IsolatedLoadingProperty(_) => "isolated_loading_property",
            Expression::JournalProperty(_) => "journal_property",
            Expression::LanguageProperty(_) => "language_property",
            Expression::EnviromentProperty(_) => "enviroment_property",
            Expression::ClusteredByProperty(_) => "clustered_by_property",
            Expression::DictProperty(_) => "dict_property",
            Expression::DictRange(_) => "dict_range",
            Expression::OnCluster(_) => "on_cluster",
            Expression::LikeProperty(_) => "like_property",
            Expression::LocationProperty(_) => "location_property",
            Expression::LockProperty(_) => "lock_property",
            Expression::LockingProperty(_) => "locking_property",
            Expression::LogProperty(_) => "log_property",
            Expression::MaterializedProperty(_) => "materialized_property",
            Expression::MergeBlockRatioProperty(_) => "merge_block_ratio_property",
            Expression::OnProperty(_) => "on_property",
            Expression::OnCommitProperty(_) => "on_commit_property",
            Expression::PartitionedByProperty(_) => "partitioned_by_property",
            Expression::PartitionByProperty(_) => "partition_by_property",
            Expression::PartitionedByBucket(_) => "partitioned_by_bucket",
            Expression::ClusterByColumnsProperty(_) => "cluster_by_columns_property",
            Expression::PartitionByTruncate(_) => "partition_by_truncate",
            Expression::PartitionByRangeProperty(_) => "partition_by_range_property",
            Expression::PartitionByRangePropertyDynamic(_) => "partition_by_range_property_dynamic",
            Expression::PartitionByListProperty(_) => "partition_by_list_property",
            Expression::PartitionList(_) => "partition_list",
            Expression::Partition(_) => "partition",
            Expression::RefreshTriggerProperty(_) => "refresh_trigger_property",
            Expression::UniqueKeyProperty(_) => "unique_key_property",
            Expression::RollupProperty(_) => "rollup_property",
            Expression::PartitionBoundSpec(_) => "partition_bound_spec",
            Expression::PartitionedOfProperty(_) => "partitioned_of_property",
            Expression::RemoteWithConnectionModelProperty(_) => {
                "remote_with_connection_model_property"
            }
            Expression::ReturnsProperty(_) => "returns_property",
            Expression::RowFormatProperty(_) => "row_format_property",
            Expression::RowFormatDelimitedProperty(_) => "row_format_delimited_property",
            Expression::RowFormatSerdeProperty(_) => "row_format_serde_property",
            Expression::QueryTransform(_) => "query_transform",
            Expression::SampleProperty(_) => "sample_property",
            Expression::SecurityProperty(_) => "security_property",
            Expression::SchemaCommentProperty(_) => "schema_comment_property",
            Expression::SemanticView(_) => "semantic_view",
            Expression::SerdeProperties(_) => "serde_properties",
            Expression::SetProperty(_) => "set_property",
            Expression::SharingProperty(_) => "sharing_property",
            Expression::SetConfigProperty(_) => "set_config_property",
            Expression::SettingsProperty(_) => "settings_property",
            Expression::SortKeyProperty(_) => "sort_key_property",
            Expression::SqlReadWriteProperty(_) => "sql_read_write_property",
            Expression::SqlSecurityProperty(_) => "sql_security_property",
            Expression::StabilityProperty(_) => "stability_property",
            Expression::StorageHandlerProperty(_) => "storage_handler_property",
            Expression::TemporaryProperty(_) => "temporary_property",
            Expression::Tags(_) => "tags",
            Expression::TransformModelProperty(_) => "transform_model_property",
            Expression::TransientProperty(_) => "transient_property",
            Expression::UsingTemplateProperty(_) => "using_template_property",
            Expression::ViewAttributeProperty(_) => "view_attribute_property",
            Expression::VolatileProperty(_) => "volatile_property",
            Expression::WithDataProperty(_) => "with_data_property",
            Expression::WithJournalTableProperty(_) => "with_journal_table_property",
            Expression::WithSchemaBindingProperty(_) => "with_schema_binding_property",
            Expression::WithSystemVersioningProperty(_) => "with_system_versioning_property",
            Expression::WithProcedureOptions(_) => "with_procedure_options",
            Expression::EncodeProperty(_) => "encode_property",
            Expression::IncludeProperty(_) => "include_property",
            Expression::Properties(_) => "properties",
            Expression::OptionsProperty(_) => "options_property",
            Expression::InputOutputFormat(_) => "input_output_format",
            Expression::Reference(_) => "reference",
            Expression::QueryOption(_) => "query_option",
            Expression::WithTableHint(_) => "with_table_hint",
            Expression::IndexTableHint(_) => "index_table_hint",
            Expression::HistoricalData(_) => "historical_data",
            Expression::Get(_) => "get",
            Expression::SetOperation(_) => "set_operation",
            Expression::Var(_) => "var",
            Expression::Variadic(_) => "variadic",
            Expression::Version(_) => "version",
            Expression::Schema(_) => "schema",
            Expression::Lock(_) => "lock",
            Expression::TableSample(_) => "table_sample",
            Expression::Tag(_) => "tag",
            Expression::UnpivotColumns(_) => "unpivot_columns",
            Expression::WindowSpec(_) => "window_spec",
            Expression::SessionParameter(_) => "session_parameter",
            Expression::PseudoType(_) => "pseudo_type",
            Expression::ObjectIdentifier(_) => "object_identifier",
            Expression::Transaction(_) => "transaction",
            Expression::Commit(_) => "commit",
            Expression::Rollback(_) => "rollback",
            Expression::AlterSession(_) => "alter_session",
            Expression::Analyze(_) => "analyze",
            Expression::AnalyzeStatistics(_) => "analyze_statistics",
            Expression::AnalyzeHistogram(_) => "analyze_histogram",
            Expression::AnalyzeSample(_) => "analyze_sample",
            Expression::AnalyzeListChainedRows(_) => "analyze_list_chained_rows",
            Expression::AnalyzeDelete(_) => "analyze_delete",
            Expression::AnalyzeWith(_) => "analyze_with",
            Expression::AnalyzeValidate(_) => "analyze_validate",
            Expression::AddPartition(_) => "add_partition",
            Expression::AttachOption(_) => "attach_option",
            Expression::DropPartition(_) => "drop_partition",
            Expression::ReplacePartition(_) => "replace_partition",
            Expression::DPipe(_) => "d_pipe",
            Expression::Operator(_) => "operator",
            Expression::PivotAny(_) => "pivot_any",
            Expression::Aliases(_) => "aliases",
            Expression::AtIndex(_) => "at_index",
            Expression::FromTimeZone(_) => "from_time_zone",
            Expression::FormatPhrase(_) => "format_phrase",
            Expression::ForIn(_) => "for_in",
            Expression::TimeUnit(_) => "time_unit",
            Expression::IntervalOp(_) => "interval_op",
            Expression::IntervalSpan(_) => "interval_span",
            Expression::HavingMax(_) => "having_max",
            Expression::CosineDistance(_) => "cosine_distance",
            Expression::DotProduct(_) => "dot_product",
            Expression::EuclideanDistance(_) => "euclidean_distance",
            Expression::ManhattanDistance(_) => "manhattan_distance",
            Expression::JarowinklerSimilarity(_) => "jarowinkler_similarity",
            Expression::Booland(_) => "booland",
            Expression::Boolor(_) => "boolor",
            Expression::ParameterizedAgg(_) => "parameterized_agg",
            Expression::ArgMax(_) => "arg_max",
            Expression::ArgMin(_) => "arg_min",
            Expression::ApproxTopK(_) => "approx_top_k",
            Expression::ApproxTopKAccumulate(_) => "approx_top_k_accumulate",
            Expression::ApproxTopKCombine(_) => "approx_top_k_combine",
            Expression::ApproxTopKEstimate(_) => "approx_top_k_estimate",
            Expression::ApproxTopSum(_) => "approx_top_sum",
            Expression::ApproxQuantiles(_) => "approx_quantiles",
            Expression::Minhash(_) => "minhash",
            Expression::FarmFingerprint(_) => "farm_fingerprint",
            Expression::Float64(_) => "float64",
            Expression::Transform(_) => "transform",
            Expression::Translate(_) => "translate",
            Expression::Grouping(_) => "grouping",
            Expression::GroupingId(_) => "grouping_id",
            Expression::Anonymous(_) => "anonymous",
            Expression::AnonymousAggFunc(_) => "anonymous_agg_func",
            Expression::CombinedAggFunc(_) => "combined_agg_func",
            Expression::CombinedParameterizedAgg(_) => "combined_parameterized_agg",
            Expression::HashAgg(_) => "hash_agg",
            Expression::Hll(_) => "hll",
            Expression::Apply(_) => "apply",
            Expression::ToBoolean(_) => "to_boolean",
            Expression::List(_) => "list",
            Expression::ToMap(_) => "to_map",
            Expression::Pad(_) => "pad",
            Expression::ToChar(_) => "to_char",
            Expression::ToNumber(_) => "to_number",
            Expression::ToDouble(_) => "to_double",
            Expression::Int64(_) => "int64",
            Expression::StringFunc(_) => "string_func",
            Expression::ToDecfloat(_) => "to_decfloat",
            Expression::TryToDecfloat(_) => "try_to_decfloat",
            Expression::ToFile(_) => "to_file",
            Expression::Columns(_) => "columns",
            Expression::ConvertToCharset(_) => "convert_to_charset",
            Expression::ConvertTimezone(_) => "convert_timezone",
            Expression::GenerateSeries(_) => "generate_series",
            Expression::AIAgg(_) => "a_i_agg",
            Expression::AIClassify(_) => "a_i_classify",
            Expression::ArrayAll(_) => "array_all",
            Expression::ArrayAny(_) => "array_any",
            Expression::ArrayConstructCompact(_) => "array_construct_compact",
            Expression::StPoint(_) => "st_point",
            Expression::StDistance(_) => "st_distance",
            Expression::StringToArray(_) => "string_to_array",
            Expression::ArraySum(_) => "array_sum",
            Expression::ObjectAgg(_) => "object_agg",
            Expression::CastToStrType(_) => "cast_to_str_type",
            Expression::CheckJson(_) => "check_json",
            Expression::CheckXml(_) => "check_xml",
            Expression::TranslateCharacters(_) => "translate_characters",
            Expression::CurrentSchemas(_) => "current_schemas",
            Expression::CurrentDatetime(_) => "current_datetime",
            Expression::Localtime(_) => "localtime",
            Expression::Localtimestamp(_) => "localtimestamp",
            Expression::Systimestamp(_) => "systimestamp",
            Expression::CurrentSchema(_) => "current_schema",
            Expression::CurrentUser(_) => "current_user",
            Expression::UtcTime(_) => "utc_time",
            Expression::UtcTimestamp(_) => "utc_timestamp",
            Expression::Timestamp(_) => "timestamp",
            Expression::DateBin(_) => "date_bin",
            Expression::Datetime(_) => "datetime",
            Expression::DatetimeAdd(_) => "datetime_add",
            Expression::DatetimeSub(_) => "datetime_sub",
            Expression::DatetimeDiff(_) => "datetime_diff",
            Expression::DatetimeTrunc(_) => "datetime_trunc",
            Expression::Dayname(_) => "dayname",
            Expression::MakeInterval(_) => "make_interval",
            Expression::PreviousDay(_) => "previous_day",
            Expression::Elt(_) => "elt",
            Expression::TimestampAdd(_) => "timestamp_add",
            Expression::TimestampSub(_) => "timestamp_sub",
            Expression::TimestampDiff(_) => "timestamp_diff",
            Expression::TimeSlice(_) => "time_slice",
            Expression::TimeAdd(_) => "time_add",
            Expression::TimeSub(_) => "time_sub",
            Expression::TimeDiff(_) => "time_diff",
            Expression::TimeTrunc(_) => "time_trunc",
            Expression::DateFromParts(_) => "date_from_parts",
            Expression::TimeFromParts(_) => "time_from_parts",
            Expression::DecodeCase(_) => "decode_case",
            Expression::Decrypt(_) => "decrypt",
            Expression::DecryptRaw(_) => "decrypt_raw",
            Expression::Encode(_) => "encode",
            Expression::Encrypt(_) => "encrypt",
            Expression::EncryptRaw(_) => "encrypt_raw",
            Expression::EqualNull(_) => "equal_null",
            Expression::ToBinary(_) => "to_binary",
            Expression::Base64DecodeBinary(_) => "base64_decode_binary",
            Expression::Base64DecodeString(_) => "base64_decode_string",
            Expression::Base64Encode(_) => "base64_encode",
            Expression::TryBase64DecodeBinary(_) => "try_base64_decode_binary",
            Expression::TryBase64DecodeString(_) => "try_base64_decode_string",
            Expression::GapFill(_) => "gap_fill",
            Expression::GenerateDateArray(_) => "generate_date_array",
            Expression::GenerateTimestampArray(_) => "generate_timestamp_array",
            Expression::GetExtract(_) => "get_extract",
            Expression::Getbit(_) => "getbit",
            Expression::OverflowTruncateBehavior(_) => "overflow_truncate_behavior",
            Expression::HexEncode(_) => "hex_encode",
            Expression::Compress(_) => "compress",
            Expression::DecompressBinary(_) => "decompress_binary",
            Expression::DecompressString(_) => "decompress_string",
            Expression::Xor(_) => "xor",
            Expression::Nullif(_) => "nullif",
            Expression::JSON(_) => "j_s_o_n",
            Expression::JSONPath(_) => "j_s_o_n_path",
            Expression::JSONPathFilter(_) => "j_s_o_n_path_filter",
            Expression::JSONPathKey(_) => "j_s_o_n_path_key",
            Expression::JSONPathRecursive(_) => "j_s_o_n_path_recursive",
            Expression::JSONPathScript(_) => "j_s_o_n_path_script",
            Expression::JSONPathSlice(_) => "j_s_o_n_path_slice",
            Expression::JSONPathSelector(_) => "j_s_o_n_path_selector",
            Expression::JSONPathSubscript(_) => "j_s_o_n_path_subscript",
            Expression::JSONPathUnion(_) => "j_s_o_n_path_union",
            Expression::Format(_) => "format",
            Expression::JSONKeys(_) => "j_s_o_n_keys",
            Expression::JSONKeyValue(_) => "j_s_o_n_key_value",
            Expression::JSONKeysAtDepth(_) => "j_s_o_n_keys_at_depth",
            Expression::JSONObject(_) => "j_s_o_n_object",
            Expression::JSONObjectAgg(_) => "j_s_o_n_object_agg",
            Expression::JSONBObjectAgg(_) => "j_s_o_n_b_object_agg",
            Expression::JSONArray(_) => "j_s_o_n_array",
            Expression::JSONArrayAgg(_) => "j_s_o_n_array_agg",
            Expression::JSONExists(_) => "j_s_o_n_exists",
            Expression::JSONColumnDef(_) => "j_s_o_n_column_def",
            Expression::JSONSchema(_) => "j_s_o_n_schema",
            Expression::JSONSet(_) => "j_s_o_n_set",
            Expression::JSONStripNulls(_) => "j_s_o_n_strip_nulls",
            Expression::JSONValue(_) => "j_s_o_n_value",
            Expression::JSONValueArray(_) => "j_s_o_n_value_array",
            Expression::JSONRemove(_) => "j_s_o_n_remove",
            Expression::JSONTable(_) => "j_s_o_n_table",
            Expression::JSONType(_) => "j_s_o_n_type",
            Expression::ObjectInsert(_) => "object_insert",
            Expression::OpenJSONColumnDef(_) => "open_j_s_o_n_column_def",
            Expression::OpenJSON(_) => "open_j_s_o_n",
            Expression::JSONBExists(_) => "j_s_o_n_b_exists",
            Expression::JSONBContains(_) => "j_s_o_n_b_contains",
            Expression::JSONBExtract(_) => "j_s_o_n_b_extract",
            Expression::JSONCast(_) => "j_s_o_n_cast",
            Expression::JSONExtract(_) => "j_s_o_n_extract",
            Expression::JSONExtractQuote(_) => "j_s_o_n_extract_quote",
            Expression::JSONExtractArray(_) => "j_s_o_n_extract_array",
            Expression::JSONExtractScalar(_) => "j_s_o_n_extract_scalar",
            Expression::JSONBExtractScalar(_) => "j_s_o_n_b_extract_scalar",
            Expression::JSONFormat(_) => "j_s_o_n_format",
            Expression::JSONBool(_) => "j_s_o_n_bool",
            Expression::JSONPathRoot(_) => "j_s_o_n_path_root",
            Expression::JSONArrayAppend(_) => "j_s_o_n_array_append",
            Expression::JSONArrayContains(_) => "j_s_o_n_array_contains",
            Expression::JSONArrayInsert(_) => "j_s_o_n_array_insert",
            Expression::ParseJSON(_) => "parse_j_s_o_n",
            Expression::ParseUrl(_) => "parse_url",
            Expression::ParseIp(_) => "parse_ip",
            Expression::ParseTime(_) => "parse_time",
            Expression::ParseDatetime(_) => "parse_datetime",
            Expression::Map(_) => "map",
            Expression::MapCat(_) => "map_cat",
            Expression::MapDelete(_) => "map_delete",
            Expression::MapInsert(_) => "map_insert",
            Expression::MapPick(_) => "map_pick",
            Expression::ScopeResolution(_) => "scope_resolution",
            Expression::Slice(_) => "slice",
            Expression::VarMap(_) => "var_map",
            Expression::MatchAgainst(_) => "match_against",
            Expression::MD5Digest(_) => "m_d5_digest",
            Expression::MD5NumberLower64(_) => "m_d5_number_lower64",
            Expression::MD5NumberUpper64(_) => "m_d5_number_upper64",
            Expression::Monthname(_) => "monthname",
            Expression::Ntile(_) => "ntile",
            Expression::Normalize(_) => "normalize",
            Expression::Normal(_) => "normal",
            Expression::Predict(_) => "predict",
            Expression::MLTranslate(_) => "m_l_translate",
            Expression::FeaturesAtTime(_) => "features_at_time",
            Expression::GenerateEmbedding(_) => "generate_embedding",
            Expression::MLForecast(_) => "m_l_forecast",
            Expression::ModelAttribute(_) => "model_attribute",
            Expression::VectorSearch(_) => "vector_search",
            Expression::Quantile(_) => "quantile",
            Expression::ApproxQuantile(_) => "approx_quantile",
            Expression::ApproxPercentileEstimate(_) => "approx_percentile_estimate",
            Expression::Randn(_) => "randn",
            Expression::Randstr(_) => "randstr",
            Expression::RangeN(_) => "range_n",
            Expression::RangeBucket(_) => "range_bucket",
            Expression::ReadCSV(_) => "read_c_s_v",
            Expression::ReadParquet(_) => "read_parquet",
            Expression::Reduce(_) => "reduce",
            Expression::RegexpExtractAll(_) => "regexp_extract_all",
            Expression::RegexpILike(_) => "regexp_i_like",
            Expression::RegexpFullMatch(_) => "regexp_full_match",
            Expression::RegexpInstr(_) => "regexp_instr",
            Expression::RegexpSplit(_) => "regexp_split",
            Expression::RegexpCount(_) => "regexp_count",
            Expression::RegrValx(_) => "regr_valx",
            Expression::RegrValy(_) => "regr_valy",
            Expression::RegrAvgy(_) => "regr_avgy",
            Expression::RegrAvgx(_) => "regr_avgx",
            Expression::RegrCount(_) => "regr_count",
            Expression::RegrIntercept(_) => "regr_intercept",
            Expression::RegrR2(_) => "regr_r2",
            Expression::RegrSxx(_) => "regr_sxx",
            Expression::RegrSxy(_) => "regr_sxy",
            Expression::RegrSyy(_) => "regr_syy",
            Expression::RegrSlope(_) => "regr_slope",
            Expression::SafeAdd(_) => "safe_add",
            Expression::SafeDivide(_) => "safe_divide",
            Expression::SafeMultiply(_) => "safe_multiply",
            Expression::SafeSubtract(_) => "safe_subtract",
            Expression::SHA2(_) => "s_h_a2",
            Expression::SHA2Digest(_) => "s_h_a2_digest",
            Expression::SortArray(_) => "sort_array",
            Expression::SplitPart(_) => "split_part",
            Expression::SubstringIndex(_) => "substring_index",
            Expression::StandardHash(_) => "standard_hash",
            Expression::StrPosition(_) => "str_position",
            Expression::Search(_) => "search",
            Expression::SearchIp(_) => "search_ip",
            Expression::StrToDate(_) => "str_to_date",
            Expression::DateStrToDate(_) => "date_str_to_date",
            Expression::DateToDateStr(_) => "date_to_date_str",
            Expression::StrToTime(_) => "str_to_time",
            Expression::StrToUnix(_) => "str_to_unix",
            Expression::StrToMap(_) => "str_to_map",
            Expression::NumberToStr(_) => "number_to_str",
            Expression::FromBase(_) => "from_base",
            Expression::Stuff(_) => "stuff",
            Expression::TimeToStr(_) => "time_to_str",
            Expression::TimeStrToTime(_) => "time_str_to_time",
            Expression::TsOrDsAdd(_) => "ts_or_ds_add",
            Expression::TsOrDsDiff(_) => "ts_or_ds_diff",
            Expression::TsOrDsToDate(_) => "ts_or_ds_to_date",
            Expression::TsOrDsToTime(_) => "ts_or_ds_to_time",
            Expression::Unhex(_) => "unhex",
            Expression::Uniform(_) => "uniform",
            Expression::UnixToStr(_) => "unix_to_str",
            Expression::UnixToTime(_) => "unix_to_time",
            Expression::Uuid(_) => "uuid",
            Expression::TimestampFromParts(_) => "timestamp_from_parts",
            Expression::TimestampTzFromParts(_) => "timestamp_tz_from_parts",
            Expression::Corr(_) => "corr",
            Expression::WidthBucket(_) => "width_bucket",
            Expression::CovarSamp(_) => "covar_samp",
            Expression::CovarPop(_) => "covar_pop",
            Expression::Week(_) => "week",
            Expression::XMLElement(_) => "x_m_l_element",
            Expression::XMLGet(_) => "x_m_l_get",
            Expression::XMLTable(_) => "x_m_l_table",
            Expression::XMLKeyValueOption(_) => "x_m_l_key_value_option",
            Expression::Zipf(_) => "zipf",
            Expression::Merge(_) => "merge",
            Expression::When(_) => "when",
            Expression::Whens(_) => "whens",
            Expression::NextValueFor(_) => "next_value_for",
            Expression::ReturnStmt(_) => "return_stmt",
        }
    }

    /// Returns the primary child expression (".this" in sqlglot).
    pub fn this(&self) -> Option<&Expression> {
        match self {
            Expression::JSONValue(j) => Some(&j.this),
            Expression::JSONTable(j) => Some(&j.this),
            Expression::JSONColumnDef(j) => j.this.as_deref(),
            Expression::Hierarchy(hierarchy) => Some(&hierarchy.source),
            Expression::Upsert(upsert) => Some(&upsert.source),
            Expression::Not(u)
            | Expression::Neg(u)
            | Expression::Pos(u)
            | Expression::BitwiseNot(u) => Some(&u.this),
            Expression::Upper(f)
            | Expression::Lower(f)
            | Expression::Length(f)
            | Expression::LTrim(f)
            | Expression::RTrim(f)
            | Expression::Reverse(f)
            | Expression::Abs(f)
            | Expression::Sqrt(f)
            | Expression::Cbrt(f)
            | Expression::Ln(f)
            | Expression::Exp(f)
            | Expression::Sign(f)
            | Expression::Date(f)
            | Expression::Time(f)
            | Expression::Initcap(f)
            | Expression::Ascii(f)
            | Expression::Chr(f)
            | Expression::Soundex(f)
            | Expression::ByteLength(f)
            | Expression::Hex(f)
            | Expression::LowerHex(f)
            | Expression::Unicode(f)
            | Expression::Typeof(f)
            | Expression::Explode(f)
            | Expression::ExplodeOuter(f)
            | Expression::MapFromEntries(f)
            | Expression::MapKeys(f)
            | Expression::MapValues(f)
            | Expression::ArrayLength(f)
            | Expression::ArraySize(f)
            | Expression::Cardinality(f)
            | Expression::ArrayReverse(f)
            | Expression::ArrayDistinct(f)
            | Expression::ArrayFlatten(f)
            | Expression::ArrayCompact(f)
            | Expression::ToArray(f)
            | Expression::JsonArrayLength(f)
            | Expression::JsonKeys(f)
            | Expression::JsonType(f)
            | Expression::ParseJson(f)
            | Expression::ToJson(f)
            | Expression::Radians(f)
            | Expression::Degrees(f)
            | Expression::Sin(f)
            | Expression::Cos(f)
            | Expression::Tan(f)
            | Expression::Asin(f)
            | Expression::Acos(f)
            | Expression::Atan(f)
            | Expression::IsNan(f)
            | Expression::IsInf(f)
            | Expression::Year(f)
            | Expression::Month(f)
            | Expression::Day(f)
            | Expression::Hour(f)
            | Expression::Minute(f)
            | Expression::Second(f)
            | Expression::DayOfWeek(f)
            | Expression::DayOfWeekIso(f)
            | Expression::DayOfMonth(f)
            | Expression::DayOfYear(f)
            | Expression::WeekOfYear(f)
            | Expression::Quarter(f)
            | Expression::Epoch(f)
            | Expression::EpochMs(f)
            | Expression::BitwiseCount(f)
            | Expression::DateFromUnixDate(f)
            | Expression::UnixDate(f)
            | Expression::UnixSeconds(f)
            | Expression::UnixMillis(f)
            | Expression::UnixMicros(f)
            | Expression::TimeStrToDate(f)
            | Expression::DateToDi(f)
            | Expression::DiToDate(f)
            | Expression::TsOrDiToDi(f)
            | Expression::TsOrDsToDatetime(f)
            | Expression::TsOrDsToTimestamp(f)
            | Expression::YearOfWeek(f)
            | Expression::YearOfWeekIso(f)
            | Expression::SHA(f)
            | Expression::SHA1Digest(f)
            | Expression::TimeToUnix(f)
            | Expression::TimeStrToUnix(f)
            | Expression::Int64(f)
            | Expression::JSONBool(f)
            | Expression::MD5NumberLower64(f)
            | Expression::MD5NumberUpper64(f)
            | Expression::DateStrToDate(f)
            | Expression::DateToDateStr(f) => Some(&f.this),
            Expression::Power(f)
            | Expression::NullIf(f)
            | Expression::IfNull(f)
            | Expression::Nvl(f)
            | Expression::Contains(f)
            | Expression::StartsWith(f)
            | Expression::EndsWith(f)
            | Expression::Levenshtein(f)
            | Expression::ModFunc(f)
            | Expression::IntDiv(f)
            | Expression::Atan2(f)
            | Expression::AddMonths(f)
            | Expression::MonthsBetween(f)
            | Expression::NextDay(f)
            | Expression::UnixToTimeStr(f)
            | Expression::ArrayContains(f)
            | Expression::ArrayPosition(f)
            | Expression::ArrayAppend(f)
            | Expression::ArrayPrepend(f)
            | Expression::ArrayUnion(f)
            | Expression::ArrayExcept(f)
            | Expression::ArrayRemove(f)
            | Expression::StarMap(f)
            | Expression::MapFromArrays(f)
            | Expression::MapContainsKey(f)
            | Expression::ElementAt(f)
            | Expression::JsonMergePatch(f)
            | Expression::JSONBContains(f)
            | Expression::JSONBExtract(f) => Some(&f.this),
            Expression::Sum(af)
            | Expression::Avg(af)
            | Expression::Min(af)
            | Expression::Max(af)
            | Expression::ArrayAgg(af)
            | Expression::CountIf(af)
            | Expression::Stddev(af)
            | Expression::StddevPop(af)
            | Expression::StddevSamp(af)
            | Expression::Variance(af)
            | Expression::VarPop(af)
            | Expression::VarSamp(af)
            | Expression::Median(af)
            | Expression::Mode(af)
            | Expression::First(af)
            | Expression::Last(af)
            | Expression::AnyValue(af)
            | Expression::ApproxDistinct(af)
            | Expression::ApproxCountDistinct(af)
            | Expression::LogicalAnd(af)
            | Expression::LogicalOr(af)
            | Expression::Skewness(af)
            | Expression::ArrayConcatAgg(af)
            | Expression::ArrayUniqueAgg(af)
            | Expression::BoolXorAgg(af)
            | Expression::BitwiseAndAgg(af)
            | Expression::BitwiseOrAgg(af)
            | Expression::BitwiseXorAgg(af) => Some(&af.this),
            Expression::And(op)
            | Expression::Or(op)
            | Expression::Add(op)
            | Expression::Sub(op)
            | Expression::Mul(op)
            | Expression::Div(op)
            | Expression::Mod(op)
            | Expression::Eq(op)
            | Expression::Neq(op)
            | Expression::Lt(op)
            | Expression::Lte(op)
            | Expression::Gt(op)
            | Expression::Gte(op)
            | Expression::BitwiseAnd(op)
            | Expression::BitwiseOr(op)
            | Expression::BitwiseXor(op)
            | Expression::Concat(op)
            | Expression::Adjacent(op)
            | Expression::TsMatch(op)
            | Expression::PropertyEQ(op)
            | Expression::ArrayContainsAll(op)
            | Expression::ArrayContainedBy(op)
            | Expression::ArrayOverlaps(op)
            | Expression::JSONBContainsAllTopKeys(op)
            | Expression::JSONBContainsAnyTopKeys(op)
            | Expression::JSONBDeleteAtPath(op)
            | Expression::ExtendsLeft(op)
            | Expression::ExtendsRight(op)
            | Expression::Is(op)
            | Expression::MemberOf(op)
            | Expression::Match(op)
            | Expression::NullSafeEq(op)
            | Expression::NullSafeNeq(op)
            | Expression::Glob(op)
            | Expression::BitwiseLeftShift(op)
            | Expression::BitwiseRightShift(op) => Some(&op.left),
            Expression::Like(op) | Expression::ILike(op) => Some(&op.left),
            Expression::Alias(a) => Some(&a.this),
            Expression::Cast(c) | Expression::TryCast(c) | Expression::SafeCast(c) => Some(&c.this),
            Expression::Paren(p) => Some(&p.this),
            Expression::Annotated(a) => Some(&a.this),
            Expression::Subquery(s) => Some(&s.this),
            Expression::Where(w) => Some(&w.this),
            Expression::Having(h) => Some(&h.this),
            Expression::Qualify(q) => Some(&q.this),
            Expression::IsNull(i) => Some(&i.this),
            Expression::Exists(e) => Some(&e.this),
            Expression::Ordered(o) => Some(&o.this),
            Expression::WindowFunction(wf) => Some(&wf.this),
            Expression::Cte(cte) => Some(&cte.this),
            Expression::Between(b) => Some(&b.this),
            Expression::In(i) => Some(&i.this),
            Expression::ReturnStmt(e) => Some(e),
            _ => None,
        }
    }

    /// Returns the secondary child expression (".expression" in sqlglot).
    pub fn expression(&self) -> Option<&Expression> {
        match self {
            Expression::And(op)
            | Expression::Or(op)
            | Expression::Add(op)
            | Expression::Sub(op)
            | Expression::Mul(op)
            | Expression::Div(op)
            | Expression::Mod(op)
            | Expression::Eq(op)
            | Expression::Neq(op)
            | Expression::Lt(op)
            | Expression::Lte(op)
            | Expression::Gt(op)
            | Expression::Gte(op)
            | Expression::BitwiseAnd(op)
            | Expression::BitwiseOr(op)
            | Expression::BitwiseXor(op)
            | Expression::Concat(op)
            | Expression::Adjacent(op)
            | Expression::TsMatch(op)
            | Expression::PropertyEQ(op)
            | Expression::ArrayContainsAll(op)
            | Expression::ArrayContainedBy(op)
            | Expression::ArrayOverlaps(op)
            | Expression::JSONBContainsAllTopKeys(op)
            | Expression::JSONBContainsAnyTopKeys(op)
            | Expression::JSONBDeleteAtPath(op)
            | Expression::ExtendsLeft(op)
            | Expression::ExtendsRight(op)
            | Expression::Is(op)
            | Expression::MemberOf(op)
            | Expression::Match(op)
            | Expression::NullSafeEq(op)
            | Expression::NullSafeNeq(op)
            | Expression::Glob(op)
            | Expression::BitwiseLeftShift(op)
            | Expression::BitwiseRightShift(op) => Some(&op.right),
            Expression::Like(op) | Expression::ILike(op) => Some(&op.right),
            Expression::Power(f)
            | Expression::NullIf(f)
            | Expression::IfNull(f)
            | Expression::Nvl(f)
            | Expression::Contains(f)
            | Expression::StartsWith(f)
            | Expression::EndsWith(f)
            | Expression::Levenshtein(f)
            | Expression::ModFunc(f)
            | Expression::IntDiv(f)
            | Expression::Atan2(f)
            | Expression::AddMonths(f)
            | Expression::MonthsBetween(f)
            | Expression::NextDay(f)
            | Expression::UnixToTimeStr(f)
            | Expression::ArrayContains(f)
            | Expression::ArrayPosition(f)
            | Expression::ArrayAppend(f)
            | Expression::ArrayPrepend(f)
            | Expression::ArrayUnion(f)
            | Expression::ArrayExcept(f)
            | Expression::ArrayRemove(f)
            | Expression::StarMap(f)
            | Expression::MapFromArrays(f)
            | Expression::MapContainsKey(f)
            | Expression::ElementAt(f)
            | Expression::JsonMergePatch(f)
            | Expression::JSONBContains(f)
            | Expression::JSONBExtract(f) => Some(&f.expression),
            _ => None,
        }
    }

    /// Returns the list of child expressions (".expressions" in sqlglot).
    pub fn expressions(&self) -> &[Expression] {
        match self {
            Expression::Select(s) => &s.expressions,
            Expression::Function(f) => &f.args,
            Expression::AggregateFunction(f) => &f.args,
            Expression::Call(f) => &f.arguments,
            Expression::From(f) => &f.expressions,
            Expression::GroupBy(g) => &g.expressions,
            Expression::In(i) => &i.expressions,
            Expression::Array(a) => &a.expressions,
            Expression::ArrayFunc(a) => &a.expressions,
            Expression::Tuple(t) => &t.expressions,
            Expression::Coalesce(f)
            | Expression::Greatest(f)
            | Expression::Least(f)
            | Expression::ArrayConcat(f)
            | Expression::ArrayIntersect(f)
            | Expression::ArrayZip(f)
            | Expression::MapConcat(f)
            | Expression::JsonArray(f) => &f.expressions,
            _ => &[],
        }
    }

    /// Returns the name of this expression as a string slice.
    pub fn name(&self) -> &str {
        match self {
            Expression::JSONValue(_) => "JSON_VALUE",
            Expression::JSONTable(_) => "JSON_TABLE",
            Expression::JsonQuery(_) => "JSON_QUERY",
            Expression::Identifier(id) => &id.name,
            Expression::Column(col) => &col.name.name,
            Expression::Table(t) => &t.name.name,
            Expression::Literal(lit) => lit.value_str(),
            Expression::Star(_) => "*",
            Expression::Function(f) => &f.name,
            Expression::AggregateFunction(f) => &f.name,
            Expression::Hierarchy(h) => &h.name,
            Expression::Alias(a) => a.this.name(),
            Expression::Boolean(b) => {
                if b.value {
                    "TRUE"
                } else {
                    "FALSE"
                }
            }
            Expression::Null(_) => "NULL",
            _ => "",
        }
    }

    /// Returns the alias name if this expression has one.
    pub fn alias_name(&self) -> &str {
        match self {
            Expression::Alias(a) => &a.alias.name,
            Expression::Table(t) => t.alias.as_ref().map(|a| a.name.as_str()).unwrap_or(""),
            Expression::Subquery(s) => s.alias.as_ref().map(|a| a.name.as_str()).unwrap_or(""),
            _ => "",
        }
    }

    /// Returns the output name of this expression (what it shows up as in a SELECT).
    pub fn output_name(&self) -> &str {
        match self {
            Expression::Alias(a) => &a.alias.name,
            Expression::Column(c) => &c.name.name,
            Expression::Identifier(id) => &id.name,
            Expression::Literal(lit) => lit.value_str(),
            Expression::Subquery(s) => s.alias.as_ref().map(|a| a.name.as_str()).unwrap_or(""),
            Expression::Star(_) => "*",
            _ => "",
        }
    }

    /// Returns comments attached to this expression.
    pub fn comments(&self) -> Vec<&str> {
        match self {
            Expression::Identifier(id) => id.trailing_comments.iter().map(|s| s.as_str()).collect(),
            Expression::Column(c) => c.trailing_comments.iter().map(|s| s.as_str()).collect(),
            Expression::Star(s) => s.trailing_comments.iter().map(|s| s.as_str()).collect(),
            Expression::Paren(p) => p.trailing_comments.iter().map(|s| s.as_str()).collect(),
            Expression::Annotated(a) => a.trailing_comments.iter().map(|s| s.as_str()).collect(),
            Expression::Alias(a) => a.trailing_comments.iter().map(|s| s.as_str()).collect(),
            Expression::Cast(c) | Expression::TryCast(c) | Expression::SafeCast(c) => {
                c.trailing_comments.iter().map(|s| s.as_str()).collect()
            }
            Expression::And(op)
            | Expression::Or(op)
            | Expression::Add(op)
            | Expression::Sub(op)
            | Expression::Mul(op)
            | Expression::Div(op)
            | Expression::Mod(op)
            | Expression::Eq(op)
            | Expression::Neq(op)
            | Expression::Lt(op)
            | Expression::Lte(op)
            | Expression::Gt(op)
            | Expression::Gte(op)
            | Expression::Concat(op)
            | Expression::BitwiseAnd(op)
            | Expression::BitwiseOr(op)
            | Expression::BitwiseXor(op) => {
                op.trailing_comments.iter().map(|s| s.as_str()).collect()
            }
            Expression::Function(f) => f.trailing_comments.iter().map(|s| s.as_str()).collect(),
            Expression::Subquery(s) => s.trailing_comments.iter().map(|s| s.as_str()).collect(),
            _ => Vec::new(),
        }
    }
}
