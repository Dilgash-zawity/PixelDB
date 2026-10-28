# PixelDB user guide

PixelDB keeps your information in **databases** and lets you build **apps** —
screens of lists, forms, boards and charts — on top of them. Everything is
stored in ordinary files on your computer; nothing is sent anywhere unless you
connect an online database yourself.

Shortcuts below say <kbd>Ctrl</kbd>; on a Mac use <kbd>⌘</kbd>. Press <kbd>?</kbd>
on any screen for its full list of shortcuts, and <kbd>Ctrl</kbd>+<kbd>K</kbd> to
search for anything.

## Getting started

1. On first launch, **choose a folder**. PixelDB keeps databases in its `DB`
   subfolder and apps in `Apps`. You can change the folder later in
   **Settings → Workspace**.
2. The home screen is a map of your apps (black tiles) and databases (grey
   tiles). Lines show which app uses which database.
3. Not sure where to begin? Click **Try a sample shop** on an empty map: it
   makes a small database of customers, products and orders with an app over
   it. The **Getting started tour** (Settings → Behaviour, or the search
   palette) walks through the main ideas.

On the map, double-click a tile to open it, right-click for more (rename,
duplicate, share, create here), and drag a tile's dot onto another tile to
connect an app to a database.

## Databases

A database page has four tabs: **Diagram**, **Datasheet**, **Query** and
**Reports** (<kbd>Ctrl</kbd>+<kbd>1</kbd> … <kbd>Ctrl</kbd>+<kbd>4</kbd>).

### Diagram: tables, fields and links

Each card is a table, like *Customers* or *Orders*.

- **Add a table**: right-click the canvas → *New table*. Name it, add fields
  (Quick add chips give you ready-made ones like Email or Price), and create it.
- **Add or change a field**: click a field, or *Add field* on a card. Name,
  type and rules (required, unique, default value, a check) are all on one page.
- **Link tables**: drag a field's dot onto another table, choose the kind
  (each order has one customer, or many-to-many), and PixelDB adds the linking
  field. Click a line to change or remove a link.
- **Indexes** make sorting and filtering large tables faster: click a table,
  then *Indexes* in its details.
- Move cards around freely; <kbd>Ctrl</kbd>+<kbd>Z</kbd> undoes a move.

Field types follow Microsoft Access: Short and Long Text, Number, Large
Number, Currency (stored exactly, never rounded), Yes/No, Date/Time,
AutoNumber, Relationship (a link to another table), Attachment, Calculated and
more.

### Datasheet: the records

Type straight into the grid, like a spreadsheet. A row is saved when you leave
it; <kbd>Esc</kbd> cancels the change you are typing.

- **Undo saved changes**: <kbd>Ctrl</kbd>+<kbd>Z</kbd> undoes the last saved
  edit, paste, import or delete; <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>Z</kbd>
  redoes it. Changing a table's structure clears the undo history for that
  database.
- **Copy and paste** blocks of cells to and from Excel or Google Sheets
  (<kbd>Ctrl</kbd>+<kbd>C</kbd> / <kbd>Ctrl</kbd>+<kbd>V</kbd>). Pasting below
  the last row adds new records.
- **Fill down**: select cells and press <kbd>Ctrl</kbd>+<kbd>D</kbd>.
- **Find and replace** (<kbd>Ctrl</kbd>+<kbd>H</kbd>) works on every record
  the current filter and search show, not just the visible page.
- **Sort, filter and search** from the toolbar. Save the combination as a
  **view** (*Views → Save current view*) to come back to it.
- **Open as form** shows one record at a time.

### Import and export

- **Import** a CSV, TSV or Excel/OpenDocument spreadsheet from *Import /
  Export → Import from a file…* (or the Diagram's canvas menu to make a new
  table from it). Pick the sheet, check how columns match fields, and see
  problems row by row before anything is written. An import is all or
  nothing, and can be undone.
- **Export** a table, a filtered view or a query result to Excel (`.xlsx`,
  with money formatted as money) or CSV from the same menu, or from the Query
  tab.

### Query: questions about your data

Pick tables and fields in the designer — PixelDB joins them along their
links — and add criteria, sorting and totals. Or switch to SQL and write a
`SELECT` yourself (queries only ever read; they never change data). **Save**
a query to use it like a table in the Datasheet, reports and apps.

### Reports: printable summaries

A report lists the records of a table or saved query, optionally **grouped**
by a field with **totals** per group and overall (count, sum, average,
smallest, largest). Choose the fields, their headings and order on the right;
the preview in the middle is exactly what prints. **Print or save as PDF…**
uses your system's print dialog. Reports are saved inside the database file.

## Apps

An app is a set of pages over one or more databases.

- **New app** on the home map: start from a template, or let PixelDB **build a
  page for each table** — a list, a form and the linked records underneath.
- Switch between **Edit** and **Run** in the top bar. In Edit mode, add widgets
  from the right panel (table, form, board, chart, number, filter, button,
  text, query), drag them on the grid, resize from the corner, and set their
  data and actions in the properties panel. Arrow keys nudge the selected
  widget; <kbd>Ctrl</kbd>+<kbd>D</kbd> duplicates it.
- **Filters and variables** connect widgets: a filter sets a variable, and
  tables or charts that filter on it update by themselves.
- With **Update automatically** on (Data tab), pages built from tables follow
  the database: new tables get pages, new fields appear in lists.
- Every change saves on its own; <kbd>Ctrl</kbd>+<kbd>Z</kbd> undoes it.

### Sharing an app

Right-click an app on the home map → **Share as a file…** (or *Share this
app as a file* in the app's search palette). PixelDB saves one `.slatepkg`
file with the app and a copy of each local database it uses. Whoever receives
it chooses **Open a shared app…** from the home map's right-click menu or
search palette; if a name is already taken there, PixelDB picks a free one and
points the app at it. Online databases are not included — the other person
needs their own connection with the same name.

## Online databases

**New database → Online** connects to PostgreSQL, MySQL, MariaDB or libSQL,
including hosted services such as Neon, Supabase, Turso, PlanetScale and
CockroachDB. Paste a connection string to fill in the details, then **Test
connection**. Passwords are stored separately from the workspace folder and
never shared. An online database has an Overview, a read-only Diagram, Data,
Structure and Query; its structure is changed with the server's own tools.
Apps use online databases just like local ones.

## Settings

- **Workspace**: the folder, and resetting the home map's layout.
- **Appearance**: light/dark, **Focus mode** (a plain grey look), accent
  colour, corners and text size.
- **Behaviour**: reduce motion (PixelDB also follows your system's setting),
  canvas hints, the tour, and resetting everything.
- **Updates**: PixelDB checks for signed updates from its official releases
  and installs them when you say so.
- **About**: the version, **Copy system info** and **Open log folder** for bug
  reports.

## When something goes wrong

Error messages have a **Copy details** button; paste that, together with
*Settings → About → Copy system info*, into a bug report. The log folder
(*Open log folder*) has more detail. Your databases are standard SQLite
files: any SQLite tool can open a copy of one.
