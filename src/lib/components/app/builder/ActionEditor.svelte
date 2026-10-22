<script lang="ts">
	import PlusIcon from '@lucide/svelte/icons/plus';
	import XIcon from '@lucide/svelte/icons/x';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import { widgetLabel, type Action, type ActionKind, type ValueRef } from '#lib/logic/app-def.js';
	import { appSession } from '#lib/state/app.svelte.js';
	import PropSelect from './PropSelect.svelte';
	import ValueRefInput from './ValueRefInput.svelte';

	let {
		action,
		context,
		columns = [],
		onchange
	}: {
		action: Action;
		/** A button's click or a table row's click (which can pass on the row's fields). */
		context: 'button' | 'row';
		columns?: string[];
		onchange: (next: Action, group?: string) => void;
	} = $props();

	const kinds: { value: ActionKind; label: string }[] = $derived(
		[
			{ value: 'none', label: 'Nothing' },
			...(context === 'row' ? [{ value: 'edit-record', label: 'Open the record to edit' }] : []),
			{ value: 'set-variable', label: 'Set a variable' },
			{ value: 'navigate', label: 'Go to a page' },
			...(context === 'button' ? [{ value: 'new-record', label: 'Start a new record' }] : []),
			{ value: 'refresh', label: 'Reload the data' },
			{ value: 'print', label: 'Print the page' }
		] as { value: ActionKind; label: string }[]
	);

	const def = $derived(appSession.def);
	const variables = $derived((def?.variables ?? []).map((v) => ({ value: v.name, label: v.name })));
	const pages = $derived((def?.pages ?? []).map((p) => ({ value: p.id, label: p.title })));
	const targets = $derived(
		(def?.pages ?? []).flatMap((p) =>
			p.widgets
				.filter((w) => w.type === 'form' || w.type === 'table')
				.map((w) => ({ value: w.id, label: w.title || widgetLabel(w.type), group: p.title }))
		)
	);
	const rowColumns = $derived(context === 'row' ? columns : undefined);
	const firstColumn = (): ValueRef =>
		context === 'row' && columns[0] ? { kind: 'column', column: columns[0] } : { kind: 'literal', value: null };

	function setKind(kind: string) {
		switch (kind as ActionKind) {
			case 'set-variable':
				return onchange({ do: 'set-variable', name: variables[0]?.value ?? '', value: firstColumn() });
			case 'navigate':
				return onchange({ do: 'navigate', page: pages[0]?.value ?? '', set: [] });
			case 'new-record':
				return onchange({ do: 'new-record', widget: targets[0]?.value ?? '' });
			default:
				return onchange({ do: kind as 'none' | 'edit-record' | 'refresh' | 'print' });
		}
	}
</script>

<div class="flex flex-col gap-3">
	<PropSelect label="Action" value={action.do} options={kinds} onchange={setKind} />

	{#if action.do === 'set-variable'}
		{@const a = action}
		<Field.Field>
			<Field.Label>Variable</Field.Label>
			<PropSelect
				value={a.name}
				options={variables}
				placeholder={variables.length ? 'Choose a variable' : 'Add a variable in Data first'}
				onchange={(v) => onchange({ ...a, name: v })}
			/>
		</Field.Field>
		<Field.Field>
			<Field.Label>To</Field.Label>
			<ValueRefInput
				value={a.value}
				columns={rowColumns}
				onchange={(v) => onchange({ ...a, value: v })}
				ontyped={(v) => onchange({ ...a, value: v }, 'action-value')}
			/>
		</Field.Field>
	{:else if action.do === 'navigate'}
		{@const a = action}
		<Field.Field>
			<Field.Label>Page</Field.Label>
			<PropSelect value={a.page} options={pages} onchange={(v) => onchange({ ...a, page: v })} />
		</Field.Field>
		<Field.Field>
			<Field.Label>And set</Field.Label>
			<div class="flex flex-col gap-2">
				{#each a.set as s, i (i)}
					<div class="flex flex-col gap-1.5 border p-2">
						<div class="flex items-center gap-1">
							<div class="min-w-0 flex-1">
								<PropSelect
									label="Variable"
									value={s.name}
									options={variables}
									onchange={(v) => onchange({ ...a, set: a.set.map((x, j) => (j === i ? { ...x, name: v } : x)) })}
								/>
							</div>
							<Button size="icon-xs" variant="ghost" aria-label="Remove" onclick={() => onchange({ ...a, set: a.set.filter((_, j) => j !== i) })}><XIcon /></Button>
						</div>
						<ValueRefInput
							value={s.value}
							columns={rowColumns}
							onchange={(v) => onchange({ ...a, set: a.set.map((x, j) => (j === i ? { ...x, value: v } : x)) })}
							ontyped={(v) => onchange({ ...a, set: a.set.map((x, j) => (j === i ? { ...x, value: v } : x)) }, `nav-${i}`)}
						/>
					</div>
				{/each}
				<Button
					size="sm"
					variant="outline"
					disabled={!variables.length}
					onclick={() => onchange({ ...a, set: [...a.set, { name: variables[0]?.value ?? '', value: firstColumn() }] })}
				>
					<PlusIcon data-icon="inline-start" /> Set a variable
				</Button>
			</div>
		</Field.Field>
	{:else if action.do === 'new-record'}
		{@const a = action}
		<Field.Field>
			<Field.Label>In</Field.Label>
			<PropSelect
				value={a.widget}
				options={targets}
				placeholder={targets.length ? 'Choose a form or table' : 'Add a form or table first'}
				onchange={(v) => onchange({ ...a, widget: v })}
			/>
		</Field.Field>
	{/if}
</div>
