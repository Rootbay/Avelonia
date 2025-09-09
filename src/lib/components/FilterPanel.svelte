<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { slide } from 'svelte/transition';

  const dispatch = createEventDispatcher();

  export let searchTerm: string;
  export let showFilters: boolean;
  export let filters: Filters;

  function clearFilters() {
    searchTerm = "";
    filters.fileType = "";
    filters.category = "";
    filters.minSize = "";
    filters.maxSize = "";
    filters.eta = "";
    filters.status = "";
    dispatch('clearFilters');
  }

  interface Filters {
    fileType: string;
    category: string;
    minSize: number | string;
    maxSize: number | string;
    eta: string;
    status: string;
  }
</script>


{#if showFilters}
  <div class="filter-settings" transition:slide>
    <h3>Filter Settings</h3>
    <div class="filter-group">
      <label for="fileType">File Type:</label>
      <input type="text" id="fileType" bind:value={filters.fileType} />
    </div>
    <div class="filter-group">
      <label for="category">Category:</label>
      <input type="text" id="category" bind:value={filters.category} />
    </div>
    <div class="filter-group">
      <label for="minSize">Min Size:</label>
      <input id="minSize" bind:value={filters.minSize} placeholder="e.g. 50MB" />
    </div>
    <div class="filter-group">
      <label for="maxSize">Max Size:</label>
      <input id="maxSize" bind:value={filters.maxSize} placeholder="e.g. 2 GB" />
    </div>
    <div class="filter-group">
      <label for="eta">ETA:</label>
      <input type="text" id="eta" bind:value={filters.eta} />
    </div>
    <div class="filter-group">
      <label for="status">Status:</label>
      <select id="status" bind:value={filters.status}>
        <option value="">All</option>
        <option value="available">Available</option>
        <option value="pending">Pending</option>
        <option value="queued">Queued</option>
        <option value="downloading">Downloading</option>
        <option value="paused">Paused</option>
        <option value="completed">Completed</option>
        <option value="failed">Failed</option>
      </select>
    </div>
    <button class="action-button" on:click={clearFilters}>Clear Filters</button>
  </div>
{/if}

<style>
  .filter-settings {
    background-color: rgba(0, 0, 0, 0.3);
    padding: 15px;
    border-radius: 8px;
    margin-bottom: 20px;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 15px;
  }

  .filter-settings h3 {
    grid-column: 1 / -1;
    margin-top: 0;
    margin-bottom: 10px;
    color: #eee;
  }

  .filter-group {
    display: flex;
    flex-direction: column;
  }

  .filter-group label {
    margin-bottom: 5px;
    font-size: 0.9em;
    color: #ccc;
  }

  .filter-group input,
  .filter-group select {
    padding: 8px;
    border-radius: 5px;
    border: 1px solid #444;
    background-color: #333;
    color: var(--white);
  }

  .action-button {
    padding: 5px 10px;
    border-radius: 5px;
    border: none;
    color: white;
    cursor: pointer;
    transition: background-color 0.2s ease-in-out;
    font-size: 0.8em;
    background-color: #007bff;
  }

  .action-button:hover {
    background-color: #0056b3;
  }
</style>
