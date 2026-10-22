// TypeScript mirrors of the Rust models in src-tauri/src/models.
// The wire format is pinned by `wire_format_tests` in src-tauri/src/models/mod.rs.

export type JsonValue = null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue };

export type CalculatedResult = 'any' | 'text' | 'integer' | 'real';
export type LookupValueType = 'integer' | 'text';

export type FieldType =
	| { type: 'short_text'; max_length?: number | null }
	| { type: 'long_text' }
	| { type: 'number'; real?: boolean }
	| { type: 'large_number' }
	| { type: 'currency'; scale?: number }
	| { type: 'auto_number' }
	| { type: 'date_time' }
	| { type: 'yes_no' }
	| { type: 'attachment' }
	| { type: 'hyperlink' }
	| { type: 'calculated'; expression: string; stored?: boolean; result?: CalculatedResult }
	| { type: 'lookup'; value_type?: LookupValueType; display_column?: string | null }
	| { type: 'unknown'; declared?: string };

export type FieldTypeName = FieldType['type'];

export type DefaultValue =
	| { kind: 'literal'; value: JsonValue }
	| { kind: 'current_timestamp' }
	| { kind: 'current_date' }
	| { kind: 'current_time' }
	| { kind: 'expression'; value: string };

export interface FieldDef {
	name: string;
	field_type: FieldType;
	required?: boolean;
	unique?: boolean;
	primary_key?: boolean;
	default?: DefaultValue | null;
	check?: string | null;
	description?: string | null;
}

export type FkAction = 'no_action' | 'restrict' | 'set_null' | 'set_default' | 'cascade';

export interface ForeignKeyDef {
	columns: string[];
	ref_table: string;
	ref_columns?: string[];
	on_delete?: FkAction;
	on_update?: FkAction;
}

export interface TableDef {
	name: string;
	fields: FieldDef[];
	foreign_keys?: ForeignKeyDef[];
	unique_constraints?: string[][];
	checks?: string[];
	description?: string | null;
}

export interface ColumnRename {
	from: string;
	to: string;
}

export interface TableDesign {
	definition: TableDef;
	renames: ColumnRename[];
}

export interface IndexInfo {
	name: string;
	unique: boolean;
	origin: string;
	partial: boolean;
	columns: string[];
}

export interface TableInfo {
	definition: TableDef;
	managed: boolean;
	unsupported_features: string[];
	schema_sql: string;
	without_rowid: boolean;
	row_count: number;
	indexes: IndexInfo[];
}

export interface SchemaChangeResult {
	rebuilt: boolean;
	warnings: string[];
}

export interface DatabaseInfo {
	path: string;
	file_size: number;
	sqlite_version: string;
	page_size: number;
	page_count: number;
	encoding: string;
	journal_mode: string;
	user_version: number;
	application_id: number;
	foreign_keys: boolean;
	table_count: number;
	view_count: number;
}

export interface ObjectSummary {
	name: string;
	kind: 'table' | 'view';
	managed: boolean;
}

export type RecordKey = { row_id: number } | { primary_key: Record<string, JsonValue> };

export interface SortSpec {
	column: string;
	descending?: boolean;
}

export type FilterOp =
	| 'eq'
	| 'ne'
	| 'lt'
	| 'le'
	| 'gt'
	| 'ge'
	| 'contains'
	| 'starts_with'
	| 'ends_with'
	| 'is_null'
	| 'is_not_null'
	| 'in';

export interface FilterSpec {
	column: string;
	op: FilterOp;
	value?: JsonValue;
}

export interface RecordQuery {
	offset?: number;
	limit?: number;
	sort?: SortSpec[];
	filters?: FilterSpec[];
	search?: string | null;
}

export interface RecordRow {
	key: RecordKey | null;
	values: JsonValue[];
}

export interface RecordPage {
	columns: string[];
	/** Field type of each column; for views, traced to the table it comes from. */
	types: (FieldType | null)[];
	rows: RecordRow[];
	total: number;
	offset: number;
	limit: number;
}

export type RecordValues = Record<string, JsonValue>;

export interface QueryResult {
	columns: string[];
	rows: JsonValue[][];
	/** True when more rows existed than were returned. */
	truncated: boolean;
	elapsed_ms: number;
	/** Field type behind each column, when it comes straight from a table. */
	types: (FieldType | null)[];
}

/** A saved query; `design` is a `VisualQuery` when it was built in the designer. */
export interface SavedQuery {
	sql: string;
	design: unknown | null;
}

export type BatchOp =
	| { op: 'insert'; values: RecordValues }
	| { op: 'update'; key: RecordKey; values: RecordValues }
	| { op: 'delete'; key: RecordKey };

export interface BatchResult {
	inserted: RecordKey[];
	updated: number;
	deleted: number;
}

export type ErrorKind =
	| 'no_database'
	| 'invalid_path'
	| 'already_exists'
	| 'not_found'
	| 'invalid_identifier'
	| 'validation'
	| 'constraint'
	| 'invalid_database'
	| 'unsupported'
	| 'confirmation_required'
	| 'sqlite'
	| 'io'
	| 'internal';

export interface WorkspaceInfo {
	path: string;
}

export interface DatabaseFile {
	name: string;
	file_name: string;
	size: number;
	modified: number | null;
	valid: boolean;
}

export interface TablePosition {
	x: number;
	y: number;
}

/** Card positions on the Relationships diagram, keyed by table name. */
export type DiagramLayout = Record<string, TablePosition>;
