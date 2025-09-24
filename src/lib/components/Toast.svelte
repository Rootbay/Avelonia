<script lang="ts">
  export let show: boolean = false;
  export let text: string = '';
  export let duration: number = 3000;

  let timer: any;
  $: if (show) {
    clearTimeout(timer);
    timer = setTimeout(() => {
      show = false;
    }, duration);
  }
  import { onDestroy } from 'svelte';
  onDestroy(() => clearTimeout(timer));
</script>

{#if show}
  <div class="toast" on:click={() => (show = false)}>
    {text}
  </div>
{/if}

<style>
  .toast {
    position: fixed;
    right: 16px;
    bottom: 16px;
    background: var(--avelonia-card);
    color: var(--avelonia-text);
    border: 1px solid var(--avelonia-border);
    border-radius: 10px;
    padding: 10px 14px;
    box-shadow: var(--shadow-card);
    z-index: 60;
    max-width: 60ch;
    cursor: pointer;
  }
</style>
