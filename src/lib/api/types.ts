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

/** A CSV or spreadsheet file read for import (models/transfer.rs): every cell as text. */
export interface ImportFile {
	file_name: string;
	sheets: ImportSheet[];
}

export interface ImportSheet {
	name: string;
	/** Rows of cells, the header row (if any) included; rows may differ in length. */
	rows: string[][];
	truncated: boolean;
}

export type ExportFormat = 'csv' | 'xlsx';

export interface ExportResult {
	file_name: string;
	rows: number;
	truncated: boolean;
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

/** What can be undone and redone (database/journal.rs), by label. */
export interface UndoState {
	undo: string | null;
	redo: string | null;
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
	| 'connection'
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

/** How the app talks to an online database's server. */
export type Engine = 'postgres' | 'mysql' | 'libsql';

/** Where an online database is; everything but its password or token. */
export interface ConnectionSettings {
	provider: string;
	engine: Engine;
	host: string;
	port: number | null;
	database: string;
	user: string;
	ssl: boolean;
}

export interface OnlineDatabase {
	name: string;
	/** Null when the connection file can't be read. */
	settings: ConnectionSettings | null;
	/** A password or token is saved for it on this computer. */
	has_secret: boolean;
	modified: number | null;
}

export interface ConnectionTest {
	server: string;
	millis: number;
}

export type OnlineObjectKind = 'table' | 'view';

/** A column of an online table, as the server describes it. */
export interface OnlineColumn {
	name: string;
	/** The server's own type, e.g. `character varying(80)`. */
	server_type: string;
	/** The closest PixelDB type, used by forms and formatting. */
	field_type: FieldType;
	nullable: boolean;
	default: string | null;
	primary_key: boolean;
	unique: boolean;
	/** Filled in by the server (identity, serial, auto_increment). */
	auto: boolean;
	/** Computed by the server; can't be written. */
	generated: boolean;
	description: string | null;
}

/** A table or view of an online database. Mirrors `online_records::OnlineTable`. */
export interface OnlineTable {
	/** The name it is listed and addressed by (`orders`, or `sales.orders` outside the default schema). */
	name: string;
	schema: string;
	kind: OnlineObjectKind;
	columns: OnlineColumn[];
	primary_key: string[];
	foreign_keys: ForeignKeyDef[];
	unique_constraints: string[][];
	/** Rows can be added, changed and deleted (tables with a primary key). */
	editable: boolean;
	row_estimate: number | null;
}

export interface OnlineInfo {
	engine: Engine;
	version: string;
	/** Bytes on disk, when the server tells. */
	size: number | null;
	schemas: string[];
	tables: number;
	views: number;
}

/** Rows read from an online database (query plans). */
export interface OnlineRows {
	columns: string[];
	types: string[];
	rows: JsonValue[][];
	truncated: boolean;
	total: number | null;
	elapsed_ms: number;
}

export interface TablePosition {
	x: number;
	y: number;
}

/** Card positions on the Relationships diagram, keyed by table name. */
export type DiagramLayout = Record<string, TablePosition>;

export type Aggregate = 'count' | 'count_distinct' | 'sum' | 'avg' | 'min' | 'max';
export type DateBucket = 'day' | 'month' | 'year';
export type AggregateOrder = 'group' | 'value_asc' | 'value_desc';

export interface GroupBy {
	column: string;
	bucket?: DateBucket | null;
}

export interface SeriesSpec {
	/** Omitted only for `count` (count of rows). */
	column?: string | null;
	agg: Aggregate;
}

/** Totals over a table or view, optionally grouped (charts and number cards). */
export interface AggregateQuery {
	group_by?: GroupBy | null;
	series: SeriesSpec[];
	filters?: FilterSpec[];
	search?: string | null;
	order?: AggregateOrder;
	limit?: number;
}

/** One row per group: the group value first when grouped, then one value per series. */
export interface AggregateResult {
	columns: string[];
	rows: JsonValue[][];
	/** Currency totals keep the currency type (minor units). */
	types: (FieldType | null)[];
	truncated: boolean;
}

/** An app file in the workspace's Apps folder. */
export interface AppFile {
	name: string;
	size: number;
	modified: number | null;
	/** Databases the app uses (empty when the file can't be read). */
	databases: string[];
	description: string;
	accent: string;
	valid: boolean;
}

/** What opening a shared app package added (package.rs `ImportedPackage`). */
export interface ImportedPackage {
	app: string;
	/** [name in the package, name it got here] */
	databases: [string, string][];
	/** Online databases the app expects; not included in the package. */
	online: string[];
}
