<script lang="ts">
	import { tick } from 'svelte';
	import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
	import ArrowRightIcon from '@lucide/svelte/icons/arrow-right';
	import KeyRoundIcon from '@lucide/svelte/icons/key-round';
	import ListPlusIcon from '@lucide/svelte/icons/list-plus';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import SparklesIcon from '@lucide/svelte/icons/sparkles';
	import Table2Icon from '@lucide/svelte/icons/table-2';
	import XIcon from '@lucide/svelte/icons/x';
	import { Badge } from '#lib/components/ui/badge/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Drawer from '#lib/components/ui/drawer/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { Spinner } from '#lib/components/ui/spinner/index.js';
	import { fieldTypeLabel, makeFieldType } from '#lib/logic/field-types.js';
	import { nextFreeName } from '#lib/logic/table-draft.js';
	import { QUICK_FIELDS, WIZARD_EXCLUDED_TYPES, fieldNameProblems, type WizardField } from '#lib/logic/table-wizard.js';
	import { session } from '#lib/state/session.svelte.js';
	import FieldTypeIcon from './FieldTypeIcon.svelte';
	import FieldTypeSelect from './FieldTypeSelect.svelte';
	import Stepper from './Stepper.svelte';

	/** Creates a table in three steps: its name, its fields, then a last look. */
	let {
		open = $bindable(false),
		initial = '',
		/** Creates the table; returns true when the drawer should close. */
		onCreate
	}: {
		open?: boolean;
		initial?: string;
		onCreate: (name: string, fields: WizardField[]) => Promise<boolean>;
	} = $props();

	const STEPS = [
		{ label: 'Name', icon: Table2Icon },
		{ label: 'Fields', icon: ListPlusIcon },
		{ label: 'Review', icon: SparklesIcon }
	];

	let step = $state(0);
	let name = $state('');
	let fields = $state<WizardField[]>([]);
	let touched = $state(false);
	let busy = $state(false);
	let nameInput = $state<HTMLInputElement | null>(null);

	// Start over each time it opens.
	$effect(() => {
		if (open) {
			step = 0;
			name = initial;
			fields = [];
			touched = false;
			tick().then(() => nameInput?.select());
		}
	});

	const nameProblem = $derived(session.tableNameProblem(name.trim()));
	const problems = $derived(fieldNameProblems(fields.map((f) => f.name)));
	const fieldsOk = $derived(problems.every((p) => p === null));
	const unused = $derived(QUICK_FIELDS.filter((q) => !fields.some((f) => f.name.trim().toLowerCase() === q.name.toLowerCase())));

	async function addField(f?: WizardField) {
		fields.push(f ?? { name: nextFreeName('Field', ['ID', ...fields.map((x) => x.name)]), type: 'short_text' });
		if (f) return;
		await tick();
		document.querySelector<HTMLInputElement>(`#nt-field-${fields.length - 1}`)?.select();
	}

	function next() {
		touched = true;
		if (step === 0 && nameProblem) return;
		if (step === 1 && !fieldsOk) return;
		touched = false;
		step++;
	}

	async function submit(e: SubmitEvent) {
		e.preventDefault();
		if (step < STEPS.length - 1) return next();
		if (busy || nameProblem || !fieldsOk) return;
		busy = true;
		try {
			if (await onCreate(name.trim(), fields.map((f) => ({ name: f.name.trim(), type: f.type })))) open = false;
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
					<Drawer.Title>New table</Drawer.Title>
					<Drawer.Description>A table holds one kind of thing, like customers or orders.</Drawer.Description>
				</div>
				<Stepper steps={STEPS} current={step} onjump={(i) => (step = i)} />
			</Drawer.Header>

			<div class="min-h-0 flex-1 overflow-y-auto px-4 pb-2">
				{#if step === 0}
					<Field.Field data-invalid={touched && !!nameProblem}>
						<Field.Label for="nt-name">What will this table hold?</Field.Label>
						<Input
							id="nt-name"
							bind:ref={nameInput}
							bind:value={name}
							placeholder="e.g. Customers"
							autocomplete="off"
							aria-invalid={touched && !!nameProblem}
						/>
						{#if touched && nameProblem}
							<Field.Error>{nameProblem}</Field.Error>
						{:else}
							<Field.Description>Use a plural name, like “Customers” or “Orders”.</Field.Description>
						{/if}
					</Field.Field>
				{:else if step === 1}
					<div class="flex flex-col gap-4">
						<div class="flex flex-col gap-2">
							<!-- The automatic ID is always there. -->
							<div class="flex items-center gap-2 rounded-md border border-dashed px-3 py-2 text-sm text-muted-foreground">
								<KeyRoundIcon class="size-4 text-warning" />
								<span class="font-medium text-foreground">ID</span>
								<span class="flex-1">numbers each record for you</span>
								<Badge variant="secondary">Auto ID</Badge>
							</div>
							{#each fields as f, i (i)}
								{@const problem = touched || f.name.trim() ? problems[i] : null}
								<Field.Field data-invalid={!!problem} class="gap-1">
									<div class="flex items-center gap-2">
										<Input
											id="nt-field-{i}"
											class="flex-1"
											bind:value={f.name}
											aria-label="Field name"
											aria-invalid={!!problem}
											autocomplete="off"
										/>
										<div class="w-44 shrink-0">
											<FieldTypeSelect value={f.type} exclude={WIZARD_EXCLUDED_TYPES} onChange={(v) => (f.type = v)} />
										</div>
										<Button variant="ghost" size="icon-sm" aria-label="Remove {f.name || 'field'}" onclick={() => fields.splice(i, 1)}>
											<XIcon />
										</Button>
									</div>
									{#if problem}<Field.Error>{problem}</Field.Error>{/if}
								</Field.Field>
							{/each}
							<Button variant="outline" class="self-start" onclick={() => addField()}>
								<PlusIcon data-icon="inline-start" /> Add field
							</Button>
						</div>
						{#if unused.length}
							<div class="flex flex-col gap-2">
								<span class="text-xs font-medium text-muted-foreground">Quick add</span>
								<div class="flex flex-wrap gap-1.5">
									{#each unused as q (q.name)}
										<Button variant="secondary" size="sm" onclick={() => addField({ ...q })}>
											<FieldTypeIcon type={q.type} data-icon="inline-start" />
											{q.name}
										</Button>
									{/each}
								</div>
							</div>
						{/if}
						<p class="text-xs text-muted-foreground">You can add more fields and links later on the Diagram.</p>
					</div>
				{:else}
					<div class="flex flex-col gap-3">
						<div class="flex items-center gap-3 rounded-lg border bg-card p-3">
							<span class="flex size-10 items-center justify-center rounded-lg bg-muted vivid:bg-tone-blue/15 vivid:text-tone-blue">
								<Table2Icon class="size-5" />
							</span>
							<div class="flex min-w-0 flex-col">
								<span class="truncate font-semibold">{name.trim()}</span>
								<span class="text-xs text-muted-foreground">{fields.length + 1} field{fields.length ? 's' : ''}</span>
							</div>
						</div>
						<ul class="flex flex-col divide-y rounded-lg border">
							<li class="flex items-center gap-2 px-3 py-2 text-sm">
								<KeyRoundIcon class="size-4 text-warning" /> ID <span class="ms-auto text-xs text-muted-foreground">Auto ID</span>
							</li>
							{#each fields as f, i (i)}
								<li class="flex items-center gap-2 px-3 py-2 text-sm">
									<FieldTypeIcon type={f.type} class="size-4 text-muted-foreground" />
									{f.name.trim()}
									<span class="ms-auto text-xs text-muted-foreground">{fieldTypeLabel(makeFieldType(f.type))}</span>
								</li>
							{/each}
						</ul>
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
				<Button type="submit" disabled={busy}>
					{#if busy}<Spinner data-icon="inline-start" />{/if}
					{#if step < STEPS.length - 1}
						{step === 1 && !fields.length ? 'Skip for now' : 'Next'} <ArrowRightIcon data-icon="inline-end" />
					{:else}
						Create table
					{/if}
				</Button>
			</Drawer.Footer>
		</form>
	</Drawer.Content>
</Drawer.Root>
