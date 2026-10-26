/**
 * English text, the reference catalog: every other language has the same keys
 * (`missingKeys` lists the gaps). Group keys by screen or component.
 */
export const en = {
	common: {
		records: { one: '{count} record', other: '{count} records' },
		values: { one: '{count} value', other: '{count} values' },
		cancel: 'Cancel',
		delete: 'Delete',
		save: 'Save',
		back: 'Back',
		next: 'Next',
		done: 'Done'
	},
	tabs: {
		diagram: 'Diagram',
		datasheet: 'Datasheet',
		query: 'Query',
		reports: 'Reports'
	},
	tour: {
		skip: 'Skip',
		start: 'Get started',
		welcome: {
			title: 'Welcome to SlateDB',
			text: 'Databases hold your information in tables. Apps put screens over them: lists, forms, boards and charts. Everything is stored in files on this computer.'
		},
		diagram: {
			title: 'Diagram: the shape of your data',
			text: 'Each card is a table, like Customers or Orders. Click a field to change it, right-click for more, and drag a field’s dot onto another table to link them.'
		},
		datasheet: {
			title: 'Datasheet: the records',
			text: 'Type straight into the grid, like a spreadsheet. Paste cells from Excel, import or export files, save filters as views, and undo saved changes with {undo}.'
		},
		query: {
			title: 'Query: questions about your data',
			text: 'Pick tables and fields to see, say, every order over 100 this month. Save a query to use it in a Datasheet or an app.'
		},
		apps: {
			title: 'Apps: screens for everyday work',
			text: 'On the home map, make an app and connect it to a database. “Build a page for each table” gives you lists and forms to start from. Press {search} anywhere to search, and ? for shortcuts.'
		}
	},
	settings: {
		title: 'Settings',
		subtitle: 'Changes save by themselves.',
		updateReady: 'An update is ready',
		workspace: { label: 'Workspace', hint: 'Folder and home map', description: 'The folder on this computer where your databases and apps live.' },
		appearance: { label: 'Appearance', hint: 'Theme, colours and text', description: 'Make SlateDB look the way you like. Changes apply straight away.' },
		behaviour: { label: 'Behaviour', hint: 'Motion, hints and the tour', description: 'How the canvases and animations feel.' },
		updates: { label: 'Updates', hint: 'New versions and fixes', description: 'Get new features and fixes from the official SlateDB releases.' },
		about: { label: 'About', hint: 'Version and bug reports', description: 'The app and who made it.' }
	},
	reports: {
		newReport: 'New report',
		none: 'No reports yet.',
		emptyTitle: 'Reports',
		emptyText: 'A report is a printable list of records, grouped and totalled the way you choose — like sales per customer.',
		print: 'Print or save as PDF…',
		firstOf: 'First {shown} of {total} records',
		total: 'Total',
		totalCount: 'Total · {records}',
		noMatch: 'No records match.',
		chooseFields: 'Choose the fields to show.',
		truncated: 'Only the first {count} records are shown. Narrow the report with a filter.',
		leftOut: 'Left out: {fields} (no longer in {source}).',
		deleteTitle: 'Delete the report “{name}”?',
		deleteText: 'Only the report’s design is deleted; your records stay.',
		deleteAction: 'Delete report'
	}
};
