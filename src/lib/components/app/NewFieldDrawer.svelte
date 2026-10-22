<script lang="ts">
	import { tick, untrack } from 'svelte';
	import { toast } from 'svelte-sonner';
	import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
	import ArrowRightIcon from '@lucide/svelte/icons/arrow-right';
	import ShapesIcon from '@lucide/svelte/icons/shapes';
	import ShieldCheckIcon from '@lucide/svelte/icons/shield-check';
	import TextCursorInputIcon from '@lucide/svelte/icons/text-cursor-input';
	import TriangleAlertIcon from '@lucide/svelte/icons/triangle-alert';
	import type { FieldDef, FieldTypeName, TableInfo } from '#lib/api/types.js';
	import * as Alert from '#lib/components/ui/alert/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Drawer from '#lib/components/ui/drawer/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { Spinner } from '#lib/components/ui/spinner/index.js';
	import { FIELD_TYPES, makeFieldType } from '#lib/logic/field-types.js';
	import { fieldDesign } from '#lib/logic/relationships.js';
	import { nextFreeName } from '#lib/logic/table-draft.js';
	import { WIZARD_EXCLUDED_TYPES, fieldNameProblems } from '#lib/logic/table-wizard.js';
	import { jsonClone } from '#lib/logic/values.js';
	import { tableService } from '#lib/services/tables.js';
	import { attempt } from '#lib/state/feedback.svelte.js';
	import { cn } from '#lib/utils.js';
	import FieldProperties from './FieldProperties.svelte';
	import FieldTypeIcon from './FieldTypeIcon.svelte';
	import Stepper from './Stepper.svelte';

	/** Adds a field to `table` in three steps: name, type, then optional rules. */
	let {
		open = $bindable(false),
		table,
		onSaved
	}: {
		open?: boolean;
		table: string;
		onSaved?: (name: string) => void;
	} = $props();

	const STEPS = [
		{ label: 'Name', icon: TextCursorInputIcon },
		{ label: 'Type', icon: ShapesIcon },
		{ label: 'Rules', icon: ShieldCheckIcon }
	];
	const TYPES = FIELD_TYPES.filter((t) => !WIZARD_EXCLUDED_TYPES.includes(t.name));

	let step = $state(0);
	let info = $state<TableInfo | null>(null);
	let edit = $state<FieldDef | null>(null);
	let touched = $state(false);
	let busy = $state(false);
	let nameInput = $state<HTMLInputElement | null>(null);

	$effect(() => {
		if (open) {
			const t = table;
			untrack(() => load(t));
		}
	});

	async function load(t: string) {
		step = 0;
		touched = false;
		info = null;
		edit = null;
		const loaded = await attempt(() => tableService.get(t));
		if (!loaded) return;
		info = loaded;
		edit = { name: nextFreeName('Field', loaded.definition.fields.map((x) => x.name)), field_type: makeFieldType('short_text') };
		await tick();
		nameInput?.select();
	}

	const blocked = $derived(
		info?.unsupported_features.length
			? `“${table}” was made by another program and uses ${info.unsupported_features.join(', ')}, so its fields can't be changed here.`
			: null
	);
	const nameProblem = $derived(
		edit && info ? fieldNameProblems([edit.name], info.definition.fields.map((f) => f.name))[0] : null
	);

	function setType(v: FieldTypeName) {
		if (edit && edit.field_type.type !== v) edit.field_type = makeFieldType(v);
	}

	async function submit(e: SubmitEvent) {
		e.preventDefault();
		if (!info || !edit || blocked || busy) return;
		if (step === 0) {
			touched = true;
			if (nameProblem) return;
		}
		if (step < STEPS.length - 1) {
			step++;
			return;
		}
		const f = jsonClone(edit);
		f.name = f.name.trim();
		const design = fieldDesign(info.definition, null, f);
		if (Array.isArray(design)) {
			toast.error('Check the field', { description: design.join(' · ') });
			return;
		}
		busy = true;
		try {
			const result = await attempt(() => tableService.saveDesign(table, design, false));
			if (!result) return;
			open = false;
			toast.success(`Added “${f.name}” to ${table}`, { description: result.warnings.join(' ') || undefined });
			onSaved?.(f.name);
		} finally {
			busy = false;
		}
	}
</script>

<Drawer.Root bind:open>
	<Drawer.Content>
		<form class="mx-auto flex min-h-0 w-full max-w-2xl flex-1 flex-col" onsubmit={submit}>
			<Drawer.Header class="gap-5">
				<div class="flex flex-col gap-1">
					<Drawer.Title>New field</Drawer.Title>
					<Drawer.Description>In {table}</Drawer.Description>
				</div>
				<Stepper steps={STEPS} current={step} onjump={(i) => (step = i)} />
			</Drawer.Header>

			<div class="min-h-0 flex-1 overflow-y-auto px-4 pb-2">
				{#if !edit}
					<div class="flex flex-col gap-3">{#each [1, 2] as i (i)}<Skeleton class="h-9" />{/each}</div>
				{:else if blocked}
					<Alert.Root class="border-warning/40 bg-warning/10 *:[svg]:text-warning">
						<TriangleAlertIcon />
						<Alert.Title class="text-warning">Read-only</Alert.Title>
						<Alert.Description>{blocked}</Alert.Description>
					</Alert.Root>
				{:else if step === 0}
					<Field.Field data-invalid={touched && !!nameProblem}>
						<Field.Label for="nf-name">What should this field be called?</Field.Label>
						<Input id="nf-name" bind:ref={nameInput} bind:value={edit.name} placeholder="e.g. Email" autocomplete="off" aria-invalid={touched && !!nameProblem} />
						{#if touched && nameProblem}
							<Field.Error>{nameProblem}</Field.Error>
						{:else}
							<Field.Description>Short and clear, like “Email” or “Due date”.</Field.Description>
						{/if}
					</Field.Field>
				{:else if step === 1}
					<div class="grid grid-cols-2 gap-2 sm:grid-cols-3" role="radiogroup" aria-label="Field type">
						{#each TYPES as t (t.name)}
							{@const selected = edit.field_type.type === t.name}
							<button
								type="button"
								role="radio"
								aria-checked={selected}
								class={cn(
									'flex flex-col items-start gap-1.5 rounded-lg border bg-card p-3 text-start outline-none transition-colors hover:bg-accent/50 focus-visible:ring-3 focus-visible:ring-ring/40',
									selected && 'border-primary bg-primary/5 ring-1 ring-primary'
								)}
								onclick={() => setType(t.name)}
								ondblclick={() => (setType(t.name), step++)}
							>
								<span class={cn('flex size-8 items-center justify-center rounded-md bg-muted', selected && 'bg-primary text-primary-foreground')}>
									<FieldTypeIcon type={t.name} class="size-4" />
								</span>
								<span class="text-sm font-medium">{t.label}</span>
								<span class="text-xs text-muted-foreground">{t.hint}</span>
							</button>
						{/each}
					</div>
				{:else}
					<div class="flex flex-col gap-4">
						<p class="text-sm text-muted-foreground">All optional — click “Add field” to keep the defaults.</p>
						<FieldProperties field={edit} />
					</div>
				{/if}
			</div>

			<Drawer.Footer class="flex-row justify-between">
				{#if step > 0}
					<Button variant="outline" onclick={() => step--}><ArrowLeftIcon data-icon="inline-start" /> Back</Button>
				{:else}
					<Drawer.Close>
						{#snippet child({ props })}<Button {...props} variant="outline">Cancel</Button>{/snippet}
					</Drawer.Close>
				{/if}
				<Button type="submit" disabled={busy || !edit || !!blocked}>
					{#if busy}<Spinner data-icon="inline-start" />{/if}
					{#if step < STEPS.length - 1}Next <ArrowRightIcon data-icon="inline-end" />{:else}Add field{/if}
				</Button>
			</Drawer.Footer>
		</form>
	</Drawer.Content>
</Drawer.Root>
