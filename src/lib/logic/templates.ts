import type { FieldDef, FieldType, RecordValues, TableDef } from '../api/types';
import { addCommand, addPage, addVariable, newApp, newWidget, type AppDef, type Layout, type Widget, type WidgetType } from './app-def';

/**
 * Starter templates: a database (tables, relationships, a few sample
 * records) and an app over it, created together from the home screen.
 */
export interface Template {
	id: string;
	name: string;
	description: string;
	/** Suggested database and app name. */
	suggestedName: string;
	/** Created in order, so parents come before the tables that link to them. */
	tables: TableDef[];
	/** Sample records per table, inserted in order (auto-numbered IDs start at 1). */
	rows: Record<string, RecordValues[]>;
	/** The app, given the name the database was created under. */
	app: (database: string) => AppDef;
}

const id = (): FieldDef => ({ name: 'ID', field_type: { type: 'auto_number' }, primary_key: true });
const f = (name: string, field_type: FieldType, extra: Partial<FieldDef> = {}): FieldDef => ({ name, field_type, ...extra });
const link = (name: string, parent: string, display: string): FieldDef =>
	f(name, { type: 'lookup', value_type: 'integer', display_column: display });
const fk = (column: string, parent: string) => ({
	columns: [column],
	ref_table: parent,
	ref_columns: ['ID'],
	on_update: 'cascade' as const,
	on_delete: 'restrict' as const
});

function widget<T extends WidgetType>(
	type: T,
	layout: Layout,
	source: string | null,
	table: string | null,
	setup: (w: Extract<Widget, { type: T }>) => void = () => {}
): Widget {
	const w = newWidget(type, layout, source && table ? { source, table } : null) as Extract<Widget, { type: T }>;
	setup(w);
	return w;
}

const contacts: Template = {
	id: 'contacts',
	name: 'Contacts',
	description: 'People and the companies they work for, with a page per company.',
	suggestedName: 'Contacts',
	tables: [
		{
			name: 'Companies',
			fields: [id(), f('Name', { type: 'short_text' }, { required: true }), f('City', { type: 'short_text' }), f('Website', { type: 'hyperlink' })]
		},
		{
			name: 'People',
			fields: [
				id(),
				f('Name', { type: 'short_text' }, { required: true }),
				f('Email', { type: 'short_text' }),
				f('Phone', { type: 'short_text' }),
				link('Company', 'Companies', 'Name'),
				f('Notes', { type: 'long_text' })
			],
			foreign_keys: [fk('Company', 'Companies')]
		}
	],
	rows: {
		Companies: [
			{ Name: 'Northwind', City: 'Seattle', Website: 'https://northwind.example' },
			{ Name: 'Contoso', City: 'London', Website: 'https://contoso.example' }
		],
		People: [
			{ Name: 'Ana Silva', Email: 'ana@northwind.example', Phone: '555-0101', Company: 1 },
			{ Name: 'Ben Okafor', Email: 'ben@contoso.example', Phone: '555-0102', Company: 2 },
			{ Name: 'Chen Wei', Email: 'chen@northwind.example', Company: 1 }
		]
	},
	app(database) {
		const app = newApp([database]);
		const s = app.sources[0].alias;
		addVariable(app, 'company', null);
		const people = app.pages[0];
		people.title = 'People';
		people.icon = 'users';
		const peopleTable = widget('table', { x: 0, y: 2, w: 7, h: 10 }, s, 'People', (w) => {
			w.columns = ['Name', 'Email', 'Phone', 'Company'];
			w.sort = { column: 'Name', descending: false };
		});
		const personForm = widget('form', { x: 7, y: 2, w: 5, h: 10 }, s, 'People', (w) => {
			w.title = 'Person';
			w.allowDelete = true;
		});
		people.widgets.push(
			widget('text', { x: 0, y: 0, w: 8, h: 2 }, null, null, (w) => (w.markdown = '# People\nEveryone you keep in touch with. Click a row to edit it.')),
			widget('stat', { x: 8, y: 0, w: 2, h: 2 }, s, 'People', (w) => (w.title = w.caption = 'People')),
			widget('stat', { x: 10, y: 0, w: 2, h: 2 }, s, 'Companies', (w) => (w.title = w.caption = 'Companies')),
			peopleTable,
			personForm
		);
		const companies = addPage(app, 'Companies', 'building');
		const companyTable = widget('table', { x: 0, y: 0, w: 5, h: 10 }, s, 'Companies', (w) => {
			w.columns = ['Name', 'City'];
			w.rowAction = { do: 'set-variable', name: 'company', value: { kind: 'column', column: 'ID' } };
		});
		const staff = widget('table', { x: 5, y: 0, w: 7, h: 9 }, s, 'People', (w) => {
			w.title = 'People at this company';
			w.columns = ['Name', 'Email', 'Phone'];
			w.filters = [{ column: 'Company', op: 'eq', value: { kind: 'variable', name: 'company' } }];
		});
		companies.widgets.push(
			companyTable,
			staff,
			widget('button', { x: 10, y: 9, w: 2, h: 1 }, null, null, (w) => {
				w.label = 'Add person';
				w.action = { do: 'new-record', widget: staff.id };
			})
		);
		addCommand(app, 'Add person').action = { do: 'new-record', widget: personForm.id };
		app.commands[0].icon = 'plus';
		app.accent = 'blue';
		app.description = 'People and the companies they work for.';
		return app;
	}
};

const shop: Template = {
	id: 'shop',
	name: 'Shop',
	description: 'Products and orders, with a sales dashboard.',
	suggestedName: 'Shop',
	tables: [
		{
			name: 'Products',
			fields: [
				id(),
				f('Name', { type: 'short_text' }, { required: true }),
				f('Category', { type: 'short_text' }),
				f('Price', { type: 'currency', scale: 2 }),
				f('In stock', { type: 'number' })
			]
		},
		{
			name: 'Orders',
			fields: [
				id(),
				link('Product', 'Products', 'Name'),
				f('Quantity', { type: 'number' }, { required: true }),
				f('Total', { type: 'currency', scale: 2 }),
				f('Placed', { type: 'date_time' }, { default: { kind: 'current_date' } })
			],
			foreign_keys: [fk('Product', 'Products')]
		}
	],
	rows: {
		Products: [
			{ Name: 'Notebook', Category: 'Paper', Price: '3.50', 'In stock': 120 },
			{ Name: 'Pen', Category: 'Writing', Price: '1.20', 'In stock': 300 },
			{ Name: 'Desk lamp', Category: 'Office', Price: '24.00', 'In stock': 15 },
			{ Name: 'Sketchbook', Category: 'Paper', Price: '8.90', 'In stock': 40 }
		],
		Orders: [
			{ Product: 1, Quantity: 4, Total: '14.00', Placed: '2026-07-03' },
			{ Product: 2, Quantity: 10, Total: '12.00', Placed: '2026-07-21' },
			{ Product: 3, Quantity: 1, Total: '24.00', Placed: '2026-08-09' },
			{ Product: 4, Quantity: 2, Total: '17.80', Placed: '2026-08-15' },
			{ Product: 1, Quantity: 6, Total: '21.00', Placed: '2026-09-02' },
			{ Product: 3, Quantity: 2, Total: '48.00', Placed: '2026-09-28' }
		]
	},
	app(database) {
		const app = newApp([database]);
		const s = app.sources[0].alias;
		addVariable(app, 'category', null);
		const dash = app.pages[0];
		dash.title = 'Dashboard';
		dash.icon = 'layout-dashboard';
		dash.widgets.push(
			widget('stat', { x: 0, y: 0, w: 4, h: 2 }, s, 'Orders', (w) => {
				w.series = { column: 'Total', agg: 'sum' };
				w.title = w.caption = 'Revenue';
			}),
			widget('stat', { x: 4, y: 0, w: 4, h: 2 }, s, 'Orders', (w) => (w.title = w.caption = 'Orders')),
			widget('stat', { x: 8, y: 0, w: 4, h: 2 }, s, 'Products', (w) => {
				w.series = { column: 'In stock', agg: 'sum' };
				w.title = w.caption = 'Items in stock';
			}),
			widget('chart', { x: 0, y: 2, w: 8, h: 7 }, s, 'Orders', (w) => {
				w.title = 'Revenue by month';
				w.groupBy = { column: 'Placed', bucket: 'month' };
				w.series = [{ column: 'Total', agg: 'sum' }];
			}),
			widget('chart', { x: 8, y: 2, w: 4, h: 7 }, s, 'Products', (w) => {
				w.title = 'Products by category';
				w.chart = 'pie';
				w.groupBy = { column: 'Category', bucket: null };
			})
		);
		const products = addPage(app, 'Products', 'package', 'Shop');
		products.widgets.push(
			widget('filter', { x: 0, y: 0, w: 3, h: 2 }, null, null, (w) => {
				w.title = 'Category';
				w.variable = 'category';
				w.control = 'select';
				w.placeholder = 'All categories';
				w.options = { data: { source: s, table: 'Products' }, valueColumn: 'Category', labelColumn: '' };
			}),
			widget('table', { x: 0, y: 2, w: 12, h: 9 }, s, 'Products', (w) => {
				w.filters = [{ column: 'Category', op: 'eq', value: { kind: 'variable', name: 'category' } }];
				w.sort = { column: 'Name', descending: false };
			})
		);
		const orders = addPage(app, 'Orders', 'receipt', 'Shop');
		orders.widgets.push(
			widget('form', { x: 0, y: 0, w: 4, h: 9 }, s, 'Orders', (w) => {
				w.title = 'Order';
				w.fields = ['Product', 'Quantity', 'Total', 'Placed'];
			}),
			widget('table', { x: 4, y: 0, w: 8, h: 9 }, s, 'Orders', (w) => {
				w.sort = { column: 'Placed', descending: true };
				w.columns = ['Placed', 'Product', 'Quantity', 'Total'];
			})
		);
		const print = addCommand(app, 'Print');
		print.icon = 'printer';
		print.action = { do: 'print' };
		app.accent = 'green';
		app.description = 'Products, orders and a sales dashboard.';
		return app;
	}
};

export const TEMPLATES: Template[] = [contacts, shop];
