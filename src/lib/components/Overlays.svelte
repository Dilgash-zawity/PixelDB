<script lang="ts">
	import { tick } from 'svelte';
	import { ui } from '../state/ui.svelte';
	import Icon, { type IconName } from './Icon.svelte';

	let promptValue = $state('');
	let promptError = $state<string | null>(null);
	let promptInput = $state<HTMLInputElement | null>(null);
	let confirmButton = $state<HTMLButtonElement | null>(null);
	let menuEl = $state<HTMLDivElement | null>(null);
	let menuPos = $state({ x: 0, y: 0 });

	// Reset and focus whenever a new dialog opens.
	$effect(() => {
		const d = ui.dialog;
		if (!d) return;
		promptError = null;
		if (d.kind === 'prompt') promptValue = d.value ?? '';
		tick().then(() => {
			if (d.kind === 'prompt') {
				promptInput?.focus();
				promptInput?.select();
			} else {
				confirmButton?.focus();
			}
		});
	});

	// Keep the context menu inside the window.
	$effect(() => {
		const m = ui.menu;
		if (!m) return;
		menuPos = { x: m.x, y: m.y };
		tick().then(() => {
			if (!menuEl) return;
			const r = menuEl.getBoundingClientRect();
			menuPos = {
				x: Math.min(m.x, window.innerWidth - r.width - 8),
				y: Math.min(m.y, window.innerHeight - r.height - 8)
			};
		});
	});

	function submitPrompt() {
		const d = ui.dialog;
		if (d?.kind !== 'prompt') return;
		const problem = d.validate?.(promptValue) ?? null;
		if (problem) {
			promptError = problem;
			return;
		}
		ui.closeDialog(promptValue);
	}

	function onKey(e: KeyboardEvent) {
		if (e.key !== 'Escape') return;
		if (ui.menu) ui.menu = null;
		else if (ui.dialog) ui.closeDialog(null);
	}
</script>

<svelte:window onkeydown={onKey} onblur={() => (ui.menu = null)} />

{#if ui.menu}
	<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
	<div class="menu-backdrop" onclick={() => (ui.menu = null)} oncontextmenu={(e) => { e.preventDefault(); ui.menu = null; }}></div>
	<div class="menu" role="menu" bind:this={menuEl} style:left="{menuPos.x}px" style:top="{menuPos.y}px">
		{#each ui.menu.items as item, i (i)}
			{#if item.separator}
				<div class="sep"></div>
			{:else}
				<button
					role="menuitem"
					class:danger={item.danger}
					disabled={item.disabled}
					onclick={() => {
						ui.menu = null;
						item.action?.();
					}}
				>
					<span class="icon">{#if item.icon}<Icon name={item.icon as IconName} size={15} />{/if}</span>
					{item.label}
				</button>
			{/if}
		{/each}
	</div>
{/if}

{#if ui.dialog}
	{@const d = ui.dialog}
	<div class="backdrop">
		<div class="dialog" role="dialog" aria-modal="true" aria-labelledby="dialog-title">
			<h3 id="dialog-title">
				{#if d.kind === 'confirm' && d.danger}<span class="warn"><Icon name="warning" size={18} /></span>{/if}
				{d.title}
			</h3>
			{#if d.kind === 'confirm'}
				<p>{d.message}</p>
				<div class="actions">
					<button onclick={() => ui.closeDialog(false)}>Cancel</button>
					<button bind:this={confirmButton} class={d.danger ? 'danger-solid' : 'primary'} onclick={() => ui.closeDialog(true)}>
						{d.confirmLabel ?? 'OK'}
					</button>
				</div>
			{:else}
				<form onsubmit={(e) => { e.preventDefault(); submitPrompt(); }}>
					<label>
						<span>{d.label}</span>
						<input bind:this={promptInput} bind:value={promptValue} oninput={() => (promptError = null)} />
					</label>
					{#if promptError}<p class="error">{promptError}</p>{/if}
					<div class="actions">
						<button type="button" onclick={() => ui.closeDialog(null)}>Cancel</button>
						<button type="submit" class="primary">{d.confirmLabel ?? 'OK'}</button>
					</div>
				</form>
			{/if}
		</div>
	</div>
{/if}

<div class="toasts" aria-live="polite">
	{#each ui.toasts as t (t.id)}
		<div class="toast {t.kind}" role={t.kind === 'error' ? 'alert' : 'status'}>
			<span class="toast-icon"><Icon name={t.kind === 'error' ? 'warning' : t.kind === 'success' ? 'check' : 'info'} size={16} /></span>
			<div>
				<strong>{t.title}</strong>
				{#if t.message}<p>{t.message}</p>{/if}
			</div>
			<button class="icon" aria-label="Dismiss" onclick={() => ui.dismiss(t.id)}><Icon name="close" size={14} /></button>
		</div>
	{/each}
</div>

<style>
	.menu-backdrop {
		position: fixed;
		inset: 0;
		z-index: 90;
	}
	.menu {
		position: fixed;
		z-index: 91;
		min-width: 190px;
		padding: 4px;
		background: var(--surface-2);
		border: 1px solid var(--border-strong);
		border-radius: var(--radius);
		box-shadow: var(--shadow);
	}
	.menu button {
		width: 100%;
		border: none;
		background: transparent;
		padding: 6px 10px 6px 6px;
		border-radius: var(--radius-sm);
	}
	.menu button:hover:not(:disabled) {
		background: var(--accent-soft);
	}
	.menu button.danger {
		color: var(--danger);
	}
	.menu .icon {
		width: 18px;
		display: inline-flex;
		justify-content: center;
		color: var(--text-muted);
	}
	.menu button.danger .icon {
		color: var(--danger);
	}
	.sep {
		height: 1px;
		margin: 4px 6px;
		background: var(--border);
	}
	.backdrop {
		position: fixed;
		inset: 0;
		z-index: 100;
		display: grid;
		place-items: center;
		background: rgba(0, 0, 0, 0.55);
	}
	.dialog {
		width: min(420px, calc(100vw - 32px));
		padding: 20px;
		background: var(--surface-2);
		border: 1px solid var(--border-strong);
		border-radius: 10px;
		box-shadow: var(--shadow);
	}
	.dialog h3 {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 15px;
		margin-bottom: 12px;
	}
	.warn {
		color: var(--warning);
	}
	.dialog p {
		margin: 0 0 4px;
		color: var(--text-muted);
	}
	.dialog label {
		display: grid;
		gap: 6px;
	}
	.dialog label span {
		color: var(--text-muted);
	}
	.dialog .error {
		margin-top: 8px;
		color: var(--danger);
	}
	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
		margin-top: 18px;
	}
	.danger-solid {
		background: var(--danger);
		border-color: var(--danger);
		color: #fff;
	}
	.toasts {
		position: fixed;
		right: 16px;
		bottom: 40px;
		z-index: 110;
		display: grid;
		gap: 8px;
		width: min(380px, calc(100vw - 32px));
	}
	.toast {
		display: grid;
		grid-template-columns: auto 1fr auto;
		gap: 10px;
		align-items: start;
		padding: 10px 10px 10px 12px;
		background: var(--surface-2);
		border: 1px solid var(--border-strong);
		border-left: 3px solid var(--text-muted);
		border-radius: var(--radius);
		box-shadow: var(--shadow);
	}
	.toast.error {
		border-left-color: var(--danger);
	}
	.toast.success {
		border-left-color: var(--success);
	}
	.toast.error .toast-icon {
		color: var(--danger);
	}
	.toast.success .toast-icon {
		color: var(--success);
	}
	.toast-icon {
		padding-top: 1px;
		color: var(--text-muted);
	}
	.toast p {
		margin: 2px 0 0;
		color: var(--text-muted);
		word-break: break-word;
	}
</style>
