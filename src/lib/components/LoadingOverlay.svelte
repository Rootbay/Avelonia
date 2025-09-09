<script lang="ts">
  export let show = false;
  export let text: string | undefined;
</script>

{#if show}
  <div class="overlay" role="status" aria-live="polite" aria-busy="true">
    <div class="spinner" aria-hidden="true"></div>
    {#if text}
      <div class="label">{text}</div>
    {/if}
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
    background: rgba(0, 0, 0, 0.35);
    z-index: 1000;
    color: var(--avelonia-text);
  }
  .spinner {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    border: 3px solid rgba(255, 255, 255, 0.2);
    border-top-color: var(--avelonia-purple);
    animation: spin 0.9s linear infinite;
  }
  .label { font-size: 0.95rem; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>

