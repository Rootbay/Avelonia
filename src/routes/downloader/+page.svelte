<script lang="ts">
  import { downloads } from "$lib/downloads";
  import FilterPanel from '$lib/components/FilterPanel.svelte';
  import DownloadItem from '$lib/components/DownloadItem.svelte';
  import { startDownload, cancelDownload } from '$lib/downloadManager';

  let searchTerm = $state("");
  let showFilters = $state(false);
  let filters = $state({
    fileType: "",
    category: "",
    minSize: "",
    maxSize: "",
    eta: "",
    status: "",
  });
  let sortBy = $state('name');
  let sortDirection = $state('asc');

  function sortDownloads(a: any, b: any) {
    let valA: any;
    let valB: any;

    switch (sortBy) {
      case 'name':
        valA = a.name.toLowerCase();
        valB = b.name.toLowerCase();
        break;
      case 'size':
        valA = parseFloat(a.size);
        valB = parseFloat(b.size);
        break;
      case 'status':
        valA = a.status.toLowerCase();
        valB = b.status.toLowerCase();
        break;
      case 'eta':
        valA = a.eta.toLowerCase();
        valB = b.eta.toLowerCase();
        break;
      case 'fileType':
        valA = a.fileType.toLowerCase();
        valB = b.fileType.toLowerCase();
        break;
      case 'category':
        valA = a.category.toLowerCase();
        valB = b.category.toLowerCase();
        break;
      default:
        valA = a.id;
        valB = b.id;
    }

    if (valA < valB) {
      return sortDirection === 'asc' ? -1 : 1;
    } else if (valA > valB) {
      return sortDirection === 'asc' ? 1 : -1;
    } else {
      return 0;
    }
  }

  const filteredDownloads = $derived(
    $downloads.filter((download) => {
      const matchesSearchTerm = download.name
        .toLowerCase()
        .includes(searchTerm.toLowerCase());
      const matchesFileType = filters.fileType
        ? download.fileType.toLowerCase().includes(filters.fileType.toLowerCase())
        : true;
      const matchesCategory = filters.category
        ? download.category.toLowerCase().includes(filters.category.toLowerCase())
        : true;

      const downloadSize = parseFloat(download.size);
      const minSize = parseFloat(filters.minSize);
      const maxSize = parseFloat(filters.maxSize);

      const matchesMinSize = filters.minSize ? downloadSize >= minSize : true;
      const matchesMaxSize = filters.maxSize ? downloadSize <= maxSize : true;

      const matchesETA = filters.eta
        ? download.eta.toLowerCase().includes(filters.eta.toLowerCase())
        : true;
      const matchesStatus = filters.status
        ? download.status.toLowerCase().includes(filters.status.toLowerCase())
        : true;

      return (
        matchesSearchTerm &&
        matchesFileType &&
        matchesCategory &&
        matchesMinSize &&
        matchesMaxSize &&
        matchesETA &&
        matchesStatus
      );
    }).sort(sortDownloads)
  );

  const totalDownloads = $derived($downloads.length);
  const availableDownloads = $derived(filteredDownloads.length);

  function handleClearFilters() {
    searchTerm = "";
    filters.fileType = "";
    filters.category = "";
    filters.minSize = "";
    filters.maxSize = "";
    filters.eta = "";
    filters.status = "";
  }
</script>

<div class="downloader-container">
  <div class="header-section">
    <p>Downloads available: {availableDownloads} / {totalDownloads}</p>
    <div class="search-filter-group">
      <input
        type="text"
        placeholder="Search downloads..."
        bind:value={searchTerm}
      />
      <select bind:value={sortBy} class="px-4 py-2 rounded-md border border-gray-700 bg-gray-800 text-white cursor-pointer transition-colors duration-200 hover:bg-gray-700">
        <option value="name">Name</option>
        <option value="size">Size</option>
        <option value="status">Status</option>
        <option value="eta">ETA</option>
        <option value="fileType">File Type</option>
        <option value="category">Category</option>
      </select>
      <button class="px-4 py-2 rounded-md border border-gray-700 bg-gray-800 text-white cursor-pointer transition-colors duration-200 hover:bg-gray-700" onclick={() => sortDirection = sortDirection === 'asc' ? 'desc' : 'asc'} aria-label={`Sort direction: ${sortDirection === 'asc' ? 'Ascending' : 'Descending'}`}>
        {sortDirection === 'asc' ? 'Asc' : 'Desc'}
      </button>
      <button class="px-4 py-2 rounded-md border-none bg-blue-500 text-white cursor-pointer transition-colors duration-200 hover:bg-blue-700" onclick={() => (showFilters = !showFilters)}
        >Filter</button
      >
    </div>
  </div>

  <FilterPanel
    bind:searchTerm={searchTerm}
    bind:showFilters={showFilters}
    bind:filters={filters}
    on:clearFilters={handleClearFilters}
  />

  <div class="program-list">
    <div class="program-list-header">
      <span></span> <!-- Icon -->
      <span>Name</span>
      <span>Size</span>
      <span>File Type</span>
      <span>Category</span>
      <span>ETA</span>
      <span>Status</span> <!-- Status -->
    </div>
    {#each filteredDownloads as download (download.id)}
      <DownloadItem
        {download}
        {startDownload}
        {cancelDownload}
      />
    {/each}
    {#if filteredDownloads.length === 0}
      <p class="no-results">No downloads found.</p>
    {/if}
  </div>
</div>

<style>
  .downloader-container {
    color: #ffffff;
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  .header-section {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 20px;
  }

  .header-section p {
    font-size: 1.1em;
    font-weight: bold;
    margin: 0;
  }

  .search-filter-group {
    display: flex;
    gap: 10px;
  }

  .header-section input {
    padding: 8px;
    border-radius: 5px;
    border: 1px solid #333;
    background-color: #2a2a2a;
    color: #ffffff;
    width: 250px;
  }

  .program-list {
    border: none;
    border-radius: 5px;
    overflow: hidden;
    background-color: rgba(0, 0, 0, 0.2);
    flex: 1;
  }

    .program-list-header {
    display: grid;
    grid-template-columns: 40px 1.5fr 1fr 1fr 1fr 1fr 1fr; /* Icon, Name, Size, File Type, Category, ETA, Status Text */
    padding: 10px;
    align-items: center;
    height: 46px;
    box-sizing: border-box;
  }

  .program-list-header span:nth-child(1) {
    justify-self: center;
  }

  .program-list-header span:nth-child(2),
  .program-list-header span:nth-child(3) {
    justify-self: start;
  }

  .program-list-header span:nth-child(4),
  .program-list-header span:nth-child(5),
  .program-list-header span:nth-child(6),
  .program-list-header span:nth-child(7) {
    justify-self: end;
  }
</style>