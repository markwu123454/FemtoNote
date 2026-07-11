<script>
  // A custom, dark-themed application menu bar (File / Edit / …) styled to match
  // Bandnote's chrome. Classic desktop behavior: click a top item to open it,
  // then hover across the bar to switch menus; click anywhere else to close.
  //
  // Menu model (passed in via `menus`):
  //   [{ label, items: [ Item, ... ] }]
  // Item:
  //   { label, hint?, action, disabled? }        — a normal command
  //   { sep: true }                               — a divider
  //   { label, checked, action }                  — a check/radio row
  //   { label, submenu: [ Item, ... ] }           — a nested menu (one level)

  export let menus = [];

  let openIndex = -1; // which top-level menu is open (-1 = none)
  let openSub = -1; // index (within the open menu's items) of the expanded submenu

  // Keyed by index, not object identity: the menu model is rebuilt on every
  // clock tick, so item references churn — indices stay stable.

  function toggle(i) {
    openIndex = openIndex === i ? -1 : i;
    openSub = -1;
  }
  function hoverTop(i) {
    // Once a menu is open, sliding across the bar switches menus (no click).
    if (openIndex !== -1 && openIndex !== i) {
      openIndex = i;
      openSub = -1;
    }
  }
  function close() {
    openIndex = -1;
    openSub = -1;
  }
  function run(item) {
    if (item.disabled || item.submenu) return;
    close();
    item.action?.();
  }
  function onKey(e) {
    if (e.key === "Escape" && openIndex !== -1) {
      e.stopPropagation();
      close();
    }
  }
</script>

<svelte:window on:keydown={onKey} />

<div class="menubar-wrap" class:open={openIndex !== -1}>
  <div class="bar">
    {#each menus as m, i}
      <div class="slot">
        <button
          class="top"
          class:active={openIndex === i}
          on:click={() => toggle(i)}
          on:mouseenter={() => hoverTop(i)}
        >{m.label}</button>

        {#if openIndex === i}
          <div class="dropdown" role="menu">
            {#each m.items as it, idx}
              {#if it.sep}
                <div class="sep" role="separator"></div>
              {:else if it.submenu}
                <!-- svelte-ignore a11y-no-static-element-interactions -->
                <div
                  class="mi has-sub"
                  class:expanded={openSub === idx}
                  role="menuitem"
                  tabindex="-1"
                  on:mouseenter={() => (openSub = idx)}
                >
                  <span class="check"></span>
                  <span class="mi-label">{it.label}</span>
                  <span class="chevron">›</span>
                  {#if openSub === idx}
                    <div class="dropdown sub" role="menu">
                      {#each it.submenu as sub}
                        {#if sub.sep}
                          <div class="sep" role="separator"></div>
                        {:else}
                          <button class="mi" role="menuitemradio" aria-checked={!!sub.checked} on:click={() => run(sub)}>
                            <span class="check">{sub.checked ? "✓" : ""}</span>
                            <span class="mi-label">{sub.label}</span>
                            {#if sub.hint}<span class="mi-hint mono">{sub.hint}</span>{/if}
                          </button>
                        {/if}
                      {/each}
                    </div>
                  {/if}
                </div>
              {:else}
                <button
                  class="mi"
                  class:disabled={it.disabled}
                  role="menuitem"
                  on:mouseenter={() => (openSub = -1)}
                  on:click={() => run(it)}
                >
                  <span class="check">{it.checked ? "✓" : ""}</span>
                  <span class="mi-label">{it.label}</span>
                  {#if it.hint}<span class="mi-hint mono">{it.hint}</span>{/if}
                </button>
              {/if}
            {/each}
          </div>
        {/if}
      </div>
    {/each}
  </div>

  {#if openIndex !== -1}
    <!-- Full-window catcher: any click outside the menus closes them. -->
    <button class="scrim" tabindex="-1" aria-hidden="true" on:click={close}></button>
  {/if}
</div>

<style>
  .menubar-wrap {
    position: relative;
    z-index: 40;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
  }
  .bar {
    position: relative;
    z-index: 2; /* above the scrim so top items stay clickable while open */
    display: flex;
    align-items: stretch;
    height: 30px;
    padding: 0 4px;
  }
  .slot {
    position: relative;
    display: flex;
  }
  .top {
    background: none;
    border: none;
    padding: 0 10px;
    cursor: default;
    color: var(--text-muted);
    font-size: var(--fs-chrome);
    border-radius: 5px;
    margin: 3px 1px;
  }
  .top:hover {
    color: var(--text-strong);
    background: var(--bg-sunken);
  }
  .top.active {
    color: var(--text-strong);
    background: var(--accent-tint);
  }

  .scrim {
    position: fixed;
    inset: 0;
    z-index: 1;
    background: transparent;
    border: none;
    margin: 0;
    padding: 0;
    cursor: default;
  }

  .dropdown {
    position: absolute;
    top: calc(100% + 3px);
    left: 2px;
    z-index: 3;
    min-width: 208px;
    padding: 5px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: var(--shadow-dialog);
    animation: slide-up 90ms ease-out;
  }
  .dropdown.sub {
    top: -6px;
    left: 100%;
    margin-left: 2px;
  }

  .mi {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    background: none;
    border: none;
    border-radius: 5px;
    padding: 6px 10px 6px 6px;
    cursor: default;
    color: var(--text);
    text-align: left;
    font-size: var(--fs-chrome);
    white-space: nowrap;
  }
  .mi:hover,
  .mi.has-sub.expanded {
    background: var(--accent-tint);
    color: var(--text-strong);
  }
  .mi.disabled {
    color: var(--text-faint);
    pointer-events: none;
  }
  .mi-label {
    flex: 1;
  }
  .mi-hint {
    color: var(--text-faint);
    font-size: 11px;
    padding-left: 18px;
  }
  .check {
    width: 14px;
    flex: none;
    text-align: center;
    color: var(--accent);
    font-size: 12px;
  }
  .chevron {
    color: var(--text-faint);
    font-size: 14px;
    line-height: 1;
  }

  .sep {
    height: 1px;
    margin: 5px 6px;
    background: var(--border);
  }
</style>
