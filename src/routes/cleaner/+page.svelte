<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { slide } from 'svelte/transition';

  import { Button } from '$lib/components/ui/button';
  import {
    Card,
    CardHeader,
    CardTitle,
    CardDescription,
    CardContent,
  } from '$lib/components/ui/card';
  import { Checkbox } from '$lib/components/ui/checkbox';
  import { Label } from '$lib/components/ui/label';
  import { Input } from '$lib/components/ui/input';
  import {
    Dialog,
    DialogContent,
    DialogHeader,
    DialogTitle,
    DialogDescription,
    DialogFooter,
  } from '$lib/components/ui/dialog';
  import { ScrollArea } from '$lib/components/ui/scroll-area';
  import { Separator } from '$lib/components/ui/separator';
  import { Skeleton } from '$lib/components/ui/skeleton/index.js';
  import { toast } from '$lib/components/ui/sonner';

  import {
    Trash2,
    RefreshCw,
    Scan,
    HardDrive,
    FolderOpen,
    Files as FilesIcon,
    Eraser,
  } from '@lucide/svelte';

  interface FileEntry {
    path: string;
    size?: number;
  }

  let tempFiles = $state<FileEntry[]>([]);
  let largeFiles = $state<FileEntry[]>([]);
  let duplicateFiles = $state<FileEntry[]>([]);
  let emptyFolders = $state<FileEntry[]>([]);
  let brokenShortcuts = $state<FileEntry[]>([]);

  let selectedTempFiles = $state<string[]>([]);
  let selectedLargeFiles = $state<string[]>([]);
  let selectedDuplicateFiles = $state<string[]>([]);
  let selectedEmptyFolders = $state<string[]>([]);
  let selectedBrokenShortcuts = $state<string[]>([]);

  let message = $state('');
  let progressMessage = $state('');
  let isLoading = $state(false);

  let eraserSelectedFiles = $state<string[]>([]);
  let eraserPasses = $state(1);
  let isErasing = $state(false);
  let eraserMessage = $state('');

  let showConfirmationModal = $state(false);
  let showRecycleBinConfirmationModal = $state(false);
  let filesToDelete = $state<string[]>([]);

  let totalDiskSpace = $state<number | null>(null);
  let availableDiskSpace = $state<number | null>(null);

  onMount(() => {
    let unlisten: (() => void) | null = null;

    listen('scan_progress', (event) => {
      progressMessage = event.payload as string;
    })
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => {});

    getDiskInfo();

    return () => {
      if (unlisten) unlisten();
    };
  });

  async function getDiskInfo() {
    try {
      const [total, available]: [number, number] = await invoke('get_drive_info');
      totalDiskSpace = total;
      availableDiskSpace = available;
    } catch (error) {
      console.error('Error getting disk info:', error);
    }
  }

  async function eraserPickFiles() {
    const result = await open({ multiple: true });
    if (Array.isArray(result)) {
      eraserSelectedFiles = result as string[];
    } else if (typeof result === 'string') {
      eraserSelectedFiles = [result];
    }
  }

  async function secureErase() {
    if (eraserSelectedFiles.length === 0) return;
    isErasing = true;
    eraserMessage = '';
    try {
      const count: number = await invoke('secure_erase', {
        files: eraserSelectedFiles,
        passes: eraserPasses,
      });
      eraserMessage = `Securely erased ${count} item(s).`;
      eraserSelectedFiles = [];
    } catch (e) {
      console.error(e);
      eraserMessage = `Failed: ${e}`;
    } finally {
      isErasing = false;
    }
  }

  function formatBytes(bytes: number, decimals = 2) {
    if (bytes === 0) return '0 Bytes';
    const k = 1024;
    const dm = decimals < 0 ? 0 : decimals;
    const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB', 'PB', 'EB', 'ZB', 'YB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i];
  }

  async function getTempFiles() {
    isLoading = true;
    message = '';
    progressMessage = '';
    try {
      message = 'Scanning for temporary files...';
      const result: string[] = await invoke('get_temp_files');
      tempFiles = result.map((path) => ({ path }));
      message = `Found ${tempFiles.length} temporary files.`;
      toast.success(message);
    } catch (error) {
      message = `Error scanning temporary files: ${error}`;
      console.error(error);
      toast.error(message);
    } finally {
      isLoading = false;
      progressMessage = '';
    }
  }

  async function deleteSelectedTempFiles() {
    if (selectedTempFiles.length === 0) return;
    confirmDeletion(selectedTempFiles);
  }

  function confirmEmptyRecycleBin() {
    showRecycleBinConfirmationModal = true;
  }

  async function emptyRecycleBin() {
    showRecycleBinConfirmationModal = false;
    isLoading = true;
    message = '';
    try {
      message = 'Emptying recycle bin...';
      await invoke('empty_recycle_bin');
      message = 'Recycle bin emptied successfully.';
      toast.success(message);
    } catch (error) {
      message = `Error emptying recycle bin: ${error}`;
      console.error(error);
      toast.error(message);
    } finally {
      isLoading = false;
    }
  }

  async function findLargeFiles() {
    isLoading = true;
    message = '';
    progressMessage = '';
    try {
      message = 'Scanning for large files...';
      const result: [string, number][] = await invoke('find_large_files');
      largeFiles = result.map(([path, size]) => ({ path, size }));
      message = `Found ${largeFiles.length} large files.`;
      toast.success(message);
    } catch (error) {
      message = `Error finding large files: ${error}`;
      console.error(error);
      toast.error(message);
    } finally {
      isLoading = false;
      progressMessage = '';
    }
  }

  async function deleteSelectedLargeFiles() {
    if (selectedLargeFiles.length === 0) return;
    confirmDeletion(selectedLargeFiles);
  }

  async function findDuplicateFiles() {
    isLoading = true;
    message = '';
    progressMessage = '';
    try {
      message = 'Scanning for duplicate files...';
      const result: [string, number][] = await invoke('find_duplicate_files');
      duplicateFiles = result.map(([path, size]) => ({ path, size }));
      message = `Found ${duplicateFiles.length} sets of duplicate files.`;
      toast.success(message);
    } catch (error) {
      message = `Error finding duplicate files: ${error}`;
      console.error(error);
      toast.error(message);
    } finally {
      isLoading = false;
      progressMessage = '';
    }
  }

  async function deleteSelectedDuplicateFiles() {
    if (selectedDuplicateFiles.length === 0) return;
    confirmDeletion(selectedDuplicateFiles);
  }

  async function findEmptyFolders() {
    isLoading = true;
    message = '';
    progressMessage = '';
    try {
      message = 'Scanning for empty folders...';
      const result: string[] = await invoke('find_empty_folders');
      emptyFolders = result.map((path) => ({ path }));
      message = `Found ${emptyFolders.length} empty folders.`;
      toast.success(message);
    } catch (error) {
      message = `Error finding empty folders: ${error}`;
      console.error(error);
      toast.error(message);
    } finally {
      isLoading = false;
      progressMessage = '';
    }
  }

  async function deleteSelectedEmptyFolders() {
    if (selectedEmptyFolders.length === 0) return;
    confirmDeletion(selectedEmptyFolders);
  }

  async function findBrokenShortcuts() {
    isLoading = true;
    message = '';
    progressMessage = '';
    try {
      message = 'Scanning for broken shortcuts...';
      const result: string[] = await invoke('find_broken_shortcuts');
      brokenShortcuts = result.map((path) => ({ path }));
      message = `Found ${brokenShortcuts.length} broken shortcuts.`;
      toast.success(message);
    } catch (error) {
      message = `Error finding broken shortcuts: ${error}`;
      console.error(error);
      toast.error(message);
    } finally {
      isLoading = false;
      progressMessage = '';
    }
  }

  async function deleteSelectedBrokenShortcuts() {
    if (selectedBrokenShortcuts.length === 0) return;
    confirmDeletion(selectedBrokenShortcuts);
  }

  function handleFileSelection(
    file: string,
    type: 'temp' | 'large' | 'duplicate' | 'empty' | 'broken_shortcut'
  ) {
    switch (type) {
      case 'temp':
        selectedTempFiles = selectedTempFiles.includes(file)
          ? selectedTempFiles.filter((f) => f !== file)
          : [...selectedTempFiles, file];
        break;
      case 'large':
        selectedLargeFiles = selectedLargeFiles.includes(file)
          ? selectedLargeFiles.filter((f) => f !== file)
          : [...selectedLargeFiles, file];
        break;
      case 'duplicate':
        selectedDuplicateFiles = selectedDuplicateFiles.includes(file)
          ? selectedDuplicateFiles.filter((f) => f !== file)
          : [...selectedDuplicateFiles, file];
        break;
      case 'empty':
        selectedEmptyFolders = selectedEmptyFolders.includes(file)
          ? selectedEmptyFolders.filter((f) => f !== file)
          : [...selectedEmptyFolders, file];
        break;
      case 'broken_shortcut':
        selectedBrokenShortcuts = selectedBrokenShortcuts.includes(file)
          ? selectedBrokenShortcuts.filter((f) => f !== file)
          : [...selectedBrokenShortcuts, file];
        break;
    }
  }

  function toggleSelectAll(type: 'temp' | 'large' | 'duplicate' | 'empty' | 'broken_shortcut') {
    switch (type) {
      case 'temp':
        selectedTempFiles =
          selectedTempFiles.length === tempFiles.length ? [] : tempFiles.map((f) => f.path);
        break;
      case 'large':
        selectedLargeFiles =
          selectedLargeFiles.length === largeFiles.length ? [] : largeFiles.map((f) => f.path);
        break;
      case 'duplicate':
        selectedDuplicateFiles =
          selectedDuplicateFiles.length === duplicateFiles.length
            ? []
            : duplicateFiles.map((f) => f.path);
        break;
      case 'empty':
        selectedEmptyFolders =
          selectedEmptyFolders.length === emptyFolders.length
            ? []
            : emptyFolders.map((f) => f.path);
        break;
      case 'broken_shortcut':
        selectedBrokenShortcuts =
          selectedBrokenShortcuts.length === brokenShortcuts.length
            ? []
            : brokenShortcuts.map((f) => f.path);
        break;
    }
  }

  function confirmDeletion(files: string[]) {
    filesToDelete = files;
    showConfirmationModal = true;
  }

  async function executeDeletion() {
    showConfirmationModal = false;
    isLoading = true;
    message = '';
    try {
      message = `Moving ${filesToDelete.length} item(s) to Trash...`;
      const deletedCount: number = await invoke('move_to_trash', { files: filesToDelete });
      message = `Moved ${deletedCount} item(s) to Trash.`;
      toast.success(message);

      tempFiles = tempFiles.filter((f) => !filesToDelete.includes(f.path));
      largeFiles = largeFiles.filter((f) => !filesToDelete.includes(f.path));
      duplicateFiles = duplicateFiles.filter((f) => !filesToDelete.includes(f.path));
      emptyFolders = emptyFolders.filter((f) => !filesToDelete.includes(f.path));
      brokenShortcuts = brokenShortcuts.filter((f) => !filesToDelete.includes(f.path));

      selectedTempFiles = [];
      selectedLargeFiles = [];
      selectedDuplicateFiles = [];
      selectedEmptyFolders = [];
      selectedBrokenShortcuts = [];
      filesToDelete = [];
    } catch (error) {
      message = `Error deleting files: ${error}`;
      console.error(error);
      toast.error(message);
    } finally {
      isLoading = false;
    }
  }

  function cancelDeletion() {
    showConfirmationModal = false;
    filesToDelete = [];
  }

  async function clearUserTemp() {
    isLoading = true;
    message = '';
    try {
      const res: any = await invoke('quick_clear_user_temp');
      message = `Cleared user temp: ${res.files_deleted} files (${formatBytes(res.bytes_deleted)}).`;
      toast.success(message);
    } catch (e) {
      console.error(e);
      message = `Failed: ${e}`;
      toast.error(message);
    } finally {
      isLoading = false;
    }
  }

  async function clearSystemTemp() {
    isLoading = true;
    message = '';
    try {
      const res: any = await invoke('quick_clear_system_temp');
      message = `Cleared system temp: ${res.files_deleted} files (${formatBytes(res.bytes_deleted)}).`;
      toast.success(message);
    } catch (e) {
      console.error(e);
      message = `Failed: ${e}`;
      toast.error(message);
    } finally {
      isLoading = false;
    }
  }

  async function clearPrefetch() {
    isLoading = true;
    message = '';
    try {
      const res: any = await invoke('quick_clear_prefetch');
      message = `Cleared Prefetch: ${res.files_deleted} files (${formatBytes(res.bytes_deleted)}).`;
      toast.success(message);
    } catch (e) {
      console.error(e);
      message = `Failed: ${e}`;
      toast.error(message);
    } finally {
      isLoading = false;
    }
  }

  async function clearRecent() {
    isLoading = true;
    message = '';
    try {
      const res: any = await invoke('quick_clear_recent');
      message = `Cleared Recent shortcuts: ${res.files_deleted} items (${formatBytes(res.bytes_deleted)}).`;
      toast.success(message);
    } catch (e) {
      console.error(e);
      message = `Failed: ${e}`;
      toast.error(message);
    } finally {
      isLoading = false;
    }
  }
</script>

<div class="space-y-6 text-foreground">
  <Card>
    <CardHeader>
      <CardTitle class="text-3xl">Cleaner</CardTitle>
      <CardDescription>Scan, select and remove clutter safely.</CardDescription>
    </CardHeader>
  </Card>

  <div>
    <h1 class="text-2xl font-bold mb-4">PC Cleaner &amp; Cloner</h1>

    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
      <Card>
        <CardHeader>
          <CardTitle class="text-xl">Quick Clean</CardTitle>
          <CardDescription>One-click cleanup for common clutter.</CardDescription>
        </CardHeader>
        <CardContent class="flex flex-wrap gap-2">
          <Button onclick={clearUserTemp} disabled={isLoading}>
            <Trash2 class="h-4 w-4" />
            Clear User Temp
          </Button>
          <Button variant="secondary" onclick={clearSystemTemp} disabled={isLoading}>
            <Trash2 class="h-4 w-4" />
            Clear System Temp
          </Button>
          <Button variant="secondary" onclick={clearPrefetch} disabled={isLoading}>
            <RefreshCw class="h-4 w-4" />
            Clear Prefetch
          </Button>
          <Button variant="secondary" onclick={clearRecent} disabled={isLoading}>
            <RefreshCw class="h-4 w-4" />
            Clear Recent
          </Button>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle class="text-xl">Temporary Files</CardTitle>
        </CardHeader>
        <CardContent class="space-y-3">
          <div class="flex flex-wrap gap-2">
            <Button onclick={getTempFiles} disabled={isLoading}>
              <Scan class="h-4 w-4" />
              Scan Temp Files
            </Button>
            <Button variant="secondary" onclick={getTempFiles} disabled={isLoading}>
              <RefreshCw class="h-4 w-4" />
              Refresh
            </Button>
            <Button
              variant="destructive"
              onclick={deleteSelectedTempFiles}
              disabled={selectedTempFiles.length === 0 || isLoading}
            >
              <Trash2 class="h-4 w-4" />
              Clean Selected ({selectedTempFiles.length})
            </Button>
          </div>

          {#if tempFiles.length > 0}
            <div transition:slide>
              <div class="flex items-center gap-2">
                <Checkbox
                  checked={selectedTempFiles.length === tempFiles.length && tempFiles.length > 0}
                  onCheckedChange={() => toggleSelectAll('temp')}
                  id="select-all-temp"
                />
                <Label for="select-all-temp" class="text-sm">Select All</Label>
              </div>

              <ScrollArea orientation="both" class="h-48 mt-2 rounded border">
                <ul class="text-sm">
                  {#each tempFiles as file (file.path)}
                    <li class="flex items-center gap-2 px-2 py-1">
                      <Checkbox
                        checked={selectedTempFiles.includes(file.path)}
                        onCheckedChange={() => handleFileSelection(file.path, 'temp')}
                      />
                      <span class="truncate">{file.path}</span>
                    </li>
                  {/each}
                </ul>
              </ScrollArea>
            </div>
          {/if}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle class="text-xl">Recycle Bin</CardTitle>
        </CardHeader>
        <CardContent>
          <Button variant="destructive" onclick={confirmEmptyRecycleBin} disabled={isLoading}>
            <Trash2 class="h-4 w-4" />
            Empty Recycle Bin
          </Button>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle class="text-xl">Disk Space</CardTitle>
        </CardHeader>
        <CardContent class="space-y-2">
          {#if totalDiskSpace !== null}
            <div class="flex items-center gap-2">
              <HardDrive class="h-4 w-4" />
              <p>Total: {formatBytes(totalDiskSpace)}</p>
            </div>
          {/if}
          {#if availableDiskSpace !== null}
            <div class="flex items-center gap-2">
              <HardDrive class="h-4 w-4" />
              <p>Available: {formatBytes(availableDiskSpace)}</p>
            </div>
          {/if}
          <Button onclick={getDiskInfo} disabled={isLoading} class="mt-2">
            <RefreshCw class="h-4 w-4" />
            Refresh Disk Info
          </Button>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle class="text-xl">Large Files (&gt;{formatBytes(100 * 1024 * 1024)})</CardTitle>
        </CardHeader>
        <CardContent class="space-y-3">
          <div class="flex flex-wrap gap-2">
            <Button onclick={findLargeFiles} disabled={isLoading}>
              <Scan class="h-4 w-4" />
              Find Large Files
            </Button>
            <Button variant="secondary" onclick={findLargeFiles} disabled={isLoading}>
              <RefreshCw class="h-4 w-4" />
              Refresh
            </Button>
            <Button
              variant="destructive"
              onclick={deleteSelectedLargeFiles}
              disabled={selectedLargeFiles.length === 0 || isLoading}
            >
              <Trash2 class="h-4 w-4" />
              Delete Selected ({selectedLargeFiles.length})
            </Button>
          </div>

          {#if largeFiles.length > 0}
            <div transition:slide>
              <div class="flex items-center gap-2">
                <Checkbox
                  checked={selectedLargeFiles.length === largeFiles.length && largeFiles.length > 0}
                  onCheckedChange={() => toggleSelectAll('large')}
                  id="select-all-large"
                />
                <Label for="select-all-large" class="text-sm">Select All</Label>
              </div>

              <ScrollArea orientation="both" class="h-48 mt-2 rounded border">
                <ul class="text-sm">
                  {#each largeFiles as file (file.path)}
                    <li class="flex items-center justify-between gap-2 px-2 py-1">
                      <div class="flex items-center gap-2">
                        <Checkbox
                          checked={selectedLargeFiles.includes(file.path)}
                          onCheckedChange={() => handleFileSelection(file.path, 'large')}
                        />
                        <span class="truncate">{file.path}</span>
                      </div>
                      {#if file.size}
                        <span class="text-xs opacity-70 whitespace-nowrap"
                          >{formatBytes(file.size)}</span
                        >
                      {/if}
                    </li>
                  {/each}
                </ul>
              </ScrollArea>
            </div>
          {/if}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle class="text-xl">Duplicate Files</CardTitle>
        </CardHeader>
        <CardContent class="space-y-3">
          <div class="flex flex-wrap gap-2">
            <Button onclick={findDuplicateFiles} disabled={isLoading}>
              <FilesIcon class="h-4 w-4" />
              Find Duplicate Files
            </Button>
            <Button variant="secondary" onclick={findDuplicateFiles} disabled={isLoading}>
              <RefreshCw class="h-4 w-4" />
              Refresh
            </Button>
            <Button
              variant="destructive"
              onclick={deleteSelectedDuplicateFiles}
              disabled={selectedDuplicateFiles.length === 0 || isLoading}
            >
              <Trash2 class="h-4 w-4" />
              Delete Selected ({selectedDuplicateFiles.length})
            </Button>
          </div>

          {#if duplicateFiles.length > 0}
            <div transition:slide>
              <div class="flex items-center gap-2">
                <Checkbox
                  checked={selectedDuplicateFiles.length === duplicateFiles.length &&
                    duplicateFiles.length > 0}
                  onCheckedChange={() => toggleSelectAll('duplicate')}
                  id="select-all-dup"
                />
                <Label for="select-all-dup" class="text-sm">Select All</Label>
              </div>

              <ScrollArea orientation="both" class="h-48 mt-2 rounded border">
                <ul class="text-sm">
                  {#each duplicateFiles as file (file.path)}
                    <li class="flex items-center gap-2 px-2 py-1">
                      <Checkbox
                        checked={selectedDuplicateFiles.includes(file.path)}
                        onCheckedChange={() => handleFileSelection(file.path, 'duplicate')}
                      />
                      <span class="truncate">{file.path}</span>
                    </li>
                  {/each}
                </ul>
              </ScrollArea>
            </div>
          {/if}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle class="text-xl">Empty Folders</CardTitle>
        </CardHeader>
        <CardContent class="space-y-3">
          <div class="flex flex-wrap gap-2">
            <Button onclick={findEmptyFolders} disabled={isLoading}>
              <FolderOpen class="h-4 w-4" />
              Find Empty Folders
            </Button>
            <Button variant="secondary" onclick={findEmptyFolders} disabled={isLoading}>
              <RefreshCw class="h-4 w-4" />
              Refresh
            </Button>
            <Button
              variant="destructive"
              onclick={deleteSelectedEmptyFolders}
              disabled={selectedEmptyFolders.length === 0 || isLoading}
            >
              <Trash2 class="h-4 w-4" />
              Delete Selected ({selectedEmptyFolders.length})
            </Button>
          </div>

          {#if emptyFolders.length > 0}
            <div transition:slide>
              <div class="flex items-center gap-2">
                <Checkbox
                  checked={selectedEmptyFolders.length === emptyFolders.length &&
                    emptyFolders.length > 0}
                  onCheckedChange={() => toggleSelectAll('empty')}
                  id="select-all-empty"
                />
                <Label for="select-all-empty" class="text-sm">Select All</Label>
              </div>

              <ScrollArea orientation="both" class="h-48 mt-2 rounded border">
                <ul class="text-sm">
                  {#each emptyFolders as folder (folder.path)}
                    <li class="flex items-center gap-2 px-2 py-1">
                      <Checkbox
                        checked={selectedEmptyFolders.includes(folder.path)}
                        onCheckedChange={() => handleFileSelection(folder.path, 'empty')}
                      />
                      <span class="truncate">{folder.path}</span>
                    </li>
                  {/each}
                </ul>
              </ScrollArea>
            </div>
          {/if}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle class="text-xl">Broken Shortcuts</CardTitle>
        </CardHeader>
        <CardContent class="space-y-3">
          <div class="flex flex-wrap gap-2">
            <Button onclick={findBrokenShortcuts} disabled={isLoading}>
              <FolderOpen class="h-4 w-4" />
              Find Broken Shortcuts
            </Button>
            <Button variant="secondary" onclick={findBrokenShortcuts} disabled={isLoading}>
              <RefreshCw class="h-4 w-4" />
              Refresh
            </Button>
            <Button
              variant="destructive"
              onclick={deleteSelectedBrokenShortcuts}
              disabled={selectedBrokenShortcuts.length === 0 || isLoading}
            >
              <Trash2 class="h-4 w-4" />
              Delete Selected ({selectedBrokenShortcuts.length})
            </Button>
          </div>

          {#if brokenShortcuts.length > 0}
            <div transition:slide>
              <div class="flex items-center gap-2">
                <Checkbox
                  checked={selectedBrokenShortcuts.length === brokenShortcuts.length &&
                    brokenShortcuts.length > 0}
                  onCheckedChange={() => toggleSelectAll('broken_shortcut')}
                  id="select-all-shortcuts"
                />
                <Label for="select-all-shortcuts" class="text-sm">Select All</Label>
              </div>

              <ScrollArea orientation="both" class="h-48 mt-2 rounded border">
                <ul class="text-sm">
                  {#each brokenShortcuts as shortcut (shortcut.path)}
                    <li class="flex items-center gap-2 px-2 py-1">
                      <Checkbox
                        checked={selectedBrokenShortcuts.includes(shortcut.path)}
                        onCheckedChange={() =>
                          handleFileSelection(shortcut.path, 'broken_shortcut')}
                      />
                      <span class="truncate">{shortcut.path}</span>
                    </li>
                  {/each}
                </ul>
              </ScrollArea>
            </div>
          {/if}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle class="text-xl">Cloner / Backup</CardTitle>
          <CardDescription
            >Features for file synchronization and backup will be implemented here.</CardDescription
          >
        </CardHeader>
        <CardContent class="space-y-2">
          <h3 class="text-lg font-medium">File Synchronization</h3>
          <Button disabled>
            <RefreshCw class="h-4 w-4" />
            Sync Folders (Coming Soon)
          </Button>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle class="text-xl">Secure Eraser</CardTitle>
          <CardDescription>Overwrite files with random data, then delete.</CardDescription>
        </CardHeader>
        <CardContent class="space-y-3">
          <div class="flex flex-wrap items-center gap-3">
            <Button onclick={eraserPickFiles} disabled={isErasing}>
              <Eraser class="h-4 w-4" />
              Choose Files
            </Button>

            <div class="flex items-center gap-2">
              <Label for="passes">Passes:</Label>
              <Input
                id="passes"
                type="number"
                min="1"
                max="7"
                bind:value={eraserPasses}
                class="w-24"
              />
            </div>

            <Button
              variant="destructive"
              onclick={secureErase}
              disabled={isErasing || eraserSelectedFiles.length === 0}
            >
              {#if isErasing}
                Erasing...
              {:else}
                <Eraser class="h-4 w-4" />
                Secure Erase
              {/if}
            </Button>
          </div>

          {#if eraserSelectedFiles.length > 0}
            <ScrollArea orientation="both" class="h-48 rounded border">
              <ul class="text-sm p-2">
                {#each eraserSelectedFiles as f (f)}
                  <li class="truncate py-1">{f}</li>
                {/each}
              </ul>
            </ScrollArea>
          {/if}

          {#if eraserMessage}
            <Separator />
            <p class="text-sm">{eraserMessage}</p>
          {/if}
        </CardContent>
      </Card>
    </div>

    {#if message || progressMessage}
      <p class="mt-4 text-center">
        {#if isLoading}
          <span class="animate-pulse">Loading...</span>
        {/if}
        {progressMessage || message}
      </p>
    {/if}
  </div>

  <Dialog open={showConfirmationModal} onOpenChange={(value) => (showConfirmationModal = value)}>
    <DialogContent>
      <DialogHeader>
        <DialogTitle>Confirm Deletion</DialogTitle>
        <DialogDescription>
          Are you sure you want to delete {filesToDelete.length} selected file(s)? This action cannot
          be undone.
        </DialogDescription>
      </DialogHeader>
      <DialogFooter class="gap-2">
        <Button variant="secondary" onclick={cancelDeletion}>Cancel</Button>
        <Button variant="destructive" onclick={executeDeletion}>
          <Trash2 class="h-4 w-4" />
          Delete
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>

  <Dialog
    open={showRecycleBinConfirmationModal}
    onOpenChange={(value) => (showRecycleBinConfirmationModal = value)}
  >
    <DialogContent>
      <DialogHeader>
        <DialogTitle>Confirm Empty Recycle Bin</DialogTitle>
        <DialogDescription>
          Are you sure you want to empty the recycle bin? This action cannot be undone.
        </DialogDescription>
      </DialogHeader>
      <DialogFooter class="gap-2">
        <Button variant="secondary" onclick={() => (showRecycleBinConfirmationModal = false)}
          >Cancel</Button
        >
        <Button variant="destructive" onclick={emptyRecycleBin}>
          <Trash2 class="h-4 w-4" />
          Empty
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>

  {#if isLoading || isErasing}
    <div
      class="fixed inset-0 z-50 flex items-center justify-center bg-background/80 backdrop-blur-sm"
      role="status"
      aria-live="polite"
      aria-busy="true"
    >
      <div class="w-[min(32rem,calc(100%-2rem))] space-y-4 rounded-lg bg-card p-6 shadow-lg">
        <div class="flex items-center gap-3">
          <Skeleton class="size-12 rounded-full" aria-hidden="true" />
          <div class="flex-1 space-y-2">
            <Skeleton class="h-4 w-3/4" aria-hidden="true" />
            <Skeleton class="h-3 w-1/2" aria-hidden="true" />
          </div>
        </div>
        <div class="space-y-2">
          <Skeleton class="h-3 w-full" aria-hidden="true" />
          <Skeleton class="h-3 w-5/6" aria-hidden="true" />
          <Skeleton class="h-3 w-4/6" aria-hidden="true" />
        </div>
        <p class="text-sm text-muted-foreground">
          {progressMessage || (isErasing ? 'Securely erasing...' : 'Working...')}
        </p>
      </div>
    </div>
  {/if}
</div>

