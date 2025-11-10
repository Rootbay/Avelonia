<script lang="ts">
  import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select';

  interface Filters {
    fileType: string;
    category: string;
    minSize: number | string;
    maxSize: number | string;
    eta: string;
    status: string;
  }

  let {
    searchTerm = $bindable(),
    showFilters = $bindable(),
    filters = $bindable(),
    onClearFilters,
    bare = false,
  }: {
    searchTerm: string;
    showFilters: boolean;
    filters: Filters;
    onClearFilters?: () => void;
    bare?: boolean;
  } = $props();

  function clearFilters() {
    searchTerm = '';
    filters.fileType = '';
    filters.category = '';
    filters.minSize = '';
    filters.maxSize = '';
    filters.eta = '';
    filters.status = '';
    onClearFilters?.();
  }
</script>

{#if showFilters}
  <Card
    class={bare
      ? 'bg-transparent border-0 shadow-none rounded-none p-0'
      : 'border border-border/60 bg-card/80 shadow-sm'}
  >
    <CardHeader
      class={(bare ? 'px-0 pt-0' : '') +
        ' flex flex-col gap-2 md:flex-row md:items-center md:justify-between'}
    >
      <CardTitle class="text-base font-semibold">Refine downloads</CardTitle>
      <Button variant="ghost" size="sm" onclick={clearFilters}>Clear all</Button>
    </CardHeader>
    <CardContent class={(bare ? 'p-0' : '') + ' grid gap-4 md:grid-cols-2 xl:grid-cols-3'}>
      <div class="space-y-2">
        <Label for="filter-file-type">File type</Label>
        <Input id="filter-file-type" bind:value={filters.fileType} placeholder="e.g. exe" />
      </div>
      <div class="space-y-2">
        <Label for="filter-category">Category</Label>
        <Input id="filter-category" bind:value={filters.category} placeholder="Utilities" />
      </div>
      <div class="space-y-2">
        <Label for="filter-min-size">Min size</Label>
        <Input id="filter-min-size" bind:value={filters.minSize} placeholder="50 MB" />
      </div>
      <div class="space-y-2">
        <Label for="filter-max-size">Max size</Label>
        <Input id="filter-max-size" bind:value={filters.maxSize} placeholder="2 GB" />
      </div>
      <div class="space-y-2">
        <Label for="filter-status">Status</Label>
        <Select type="single" bind:value={filters.status}>
          <SelectTrigger id="filter-status" class="w-full">
            <p>
              {filters.status === 'available'
                ? 'Available'
                : filters.status === 'pending'
                ? 'Pending'
                : filters.status === 'queued'
                ? 'Queued'
                : filters.status === 'downloading'
                ? 'Downloading'
                : filters.status === 'paused'
                ? 'Paused'
                : filters.status === 'completed'
                ? 'Completed'
                : filters.status === 'installed'
                ? 'Installed'
                : filters.status === 'failed'
                ? 'Failed'
                : 'All statuses'}
            </p>
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="">All statuses</SelectItem>
            <SelectItem value="available">Available</SelectItem>
            <SelectItem value="pending">Pending</SelectItem>
            <SelectItem value="queued">Queued</SelectItem>
            <SelectItem value="downloading">Downloading</SelectItem>
            <SelectItem value="paused">Paused</SelectItem>
            <SelectItem value="completed">Completed</SelectItem>
            <SelectItem value="installed">Installed</SelectItem>
            <SelectItem value="failed">Failed</SelectItem>
          </SelectContent>
        </Select>
      </div>
      <Button
        variant="secondary"
        size="sm"
        class="md:col-span-2 xl:col-span-3 w-fit"
        onclick={clearFilters}
      >
        Reset filters
      </Button>
    </CardContent>
  </Card>
{/if}
