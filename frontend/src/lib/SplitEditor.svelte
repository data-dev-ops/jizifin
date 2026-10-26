<script context="module">
  /** Editor value to the API's `[{ member, pct }]`. */
  export function toShares(map) {
    return Object.entries(map)
      .filter(([, pct]) => pct !== '' && pct !== null && pct !== undefined)
      .map(([member, pct]) => ({ member: Number(member), pct: Number(pct) }));
  }

  /** API shares to the editor value. */
  export function fromShares(list = []) {
    return Object.fromEntries(list.map((s) => [s.member, s.pct]));
  }
</script>

<script>
  /**
   * SplitEditor.svelte — percentage shares per member.
   * Two members: one linked slider. More: one input each. The server validates
   * that shares total 100 and answers 422 with the reason otherwise.
   */
  export let members = [];
  /** `{ [memberId]: pct }` */
  export let shares = {};
  export let idPrefix = 'split';

  let useSlider = true;

  $: pair = members.length === 2 && useSlider;
  $: left = shares[members[0]?.id] ?? 50;

  function slide(e) {
    const pct = Number(e.target.value);
    shares = { [members[0].id]: pct, [members[1].id]: 100 - pct };
  }
</script>

<div class="p-3 bg-neutral-50 dark:bg-neutral-900 border border-neutral-200 dark:border-indigo-800/40 rounded-xl space-y-2">
  {#if pair}
    <div class="flex items-center justify-between gap-4 py-2">
      <div class="text-right w-24 flex-shrink-0">
        <span class="text-xs font-semibold block truncate" style="color: {members[0].color}">{members[0].name}</span>
        <span class="text-lg font-bold text-neutral-900 dark:text-neutral-100">{left}%</span>
      </div>
      <input
        id="{idPrefix}-slider"
        type="range" min="0" max="100" step="1" value={left} on:input={slide}
        class="flex-1 h-2 rounded-lg appearance-none cursor-pointer accent-indigo-600"
        style="background: linear-gradient(to right, {members[0].color} 0%, {members[0].color} {left}%, {members[1].color} {left}%, {members[1].color} 100%);"
      />
      <div class="text-left w-24 flex-shrink-0">
        <span class="text-xs font-semibold block truncate" style="color: {members[1].color}">{members[1].name}</span>
        <span class="text-lg font-bold text-neutral-900 dark:text-neutral-100">{100 - left}%</span>
      </div>
    </div>
    <button type="button" on:click={() => (useSlider = false)} class="text-[10px] text-neutral-500 hover:text-neutral-800 dark:hover:text-neutral-300 underline">
      Switch to manual inputs
    </button>
  {:else}
    {#each members as m (m.id)}
      <div class="flex items-center gap-3">
        <span class="text-xs font-semibold w-20 truncate" style="color: {m.color}">{m.name}</span>
        <input
          id="{idPrefix}-pct-{m.id}"
          type="number" min="0" max="100" step="0.01"
          bind:value={shares[m.id]}
          class="w-24 bg-white dark:bg-neutral-800 border border-neutral-300 dark:border-neutral-700 rounded-lg px-2 py-1.5 text-sm text-neutral-900 dark:text-neutral-100 focus:outline-none focus:ring-1"
        />
        <span class="text-xs text-neutral-500">%</span>
      </div>
    {/each}
    <p class="text-[10px] text-neutral-500">Shares must total 100%.</p>
    {#if members.length === 2}
      <button type="button" on:click={() => (useSlider = true)} class="text-[10px] text-neutral-500 hover:text-neutral-800 dark:hover:text-neutral-300 underline">
        Switch to slider
      </button>
    {/if}
  {/if}
</div>
