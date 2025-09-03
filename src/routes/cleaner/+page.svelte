<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';

  interface FileEntry {
    path: string;
    size?: number;
  }

  let tempFiles: FileEntry[] = [];
  let largeFiles: FileEntry[] = [];
  let duplicateFiles: FileEntry[] = [];
  let emptyFolders: FileEntry[] = [];
  let brokenShortcuts: FileEntry[] = [];

  let selectedTempFiles: string[] = [];
  let selectedLargeFiles: string[] = [];
  let selectedDuplicateFiles: string[] = [];
  let selectedEmptyFolders: string[] = [];
  let selectedBrokenShortcuts: string[] = [];

  let message: string = '';
  let progressMessage: string = '';
  let isLoading: boolean = false;

  let showConfirmationModal: boolean = false;
  let showRecycleBinConfirmationModal: boolean = false;
  let filesToDelete: string[] = [];

  let totalDiskSpace: number = 0;
  let availableDiskSpace: number = 0;

  onMount(() => {
    let unlisten: (() => void) | null = null;

    // Attach listener; ignore the returned promise for onMount's return type
    listen('scan_progress', (event) => {
      progressMessage = event.payload as string;
    }).then((fn) => {
      unlisten = fn;
    }).catch(() => {
      // no-op if event bridge isn't available
    });

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
      tempFiles = result.map(path => ({ path }));
      message = `Found ${tempFiles.length} temporary files.`;
    } catch (error) {
      message = `Error scanning temporary files: ${error}`;
      console.error(error);
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
    } catch (error) {
      message = `Error emptying recycle bin: ${error}`;
      console.error(error);
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
    } catch (error) {
      message = `Error finding large files: ${error}`;
      console.error(error);
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
    } catch (error) {
      message = `Error finding duplicate files: ${error}`;
      console.error(error);
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
      emptyFolders = result.map(path => ({ path }));
      message = `Found ${emptyFolders.length} empty folders.`;
    } catch (error) {
      message = `Error finding empty folders: ${error}`;
      console.error(error);
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
      brokenShortcuts = result.map(path => ({ path }));
      message = `Found ${brokenShortcuts.length} broken shortcuts.`;
    } catch (error) {
      message = `Error finding broken shortcuts: ${error}`;
      console.error(error);
    } finally {
      isLoading = false;
      progressMessage = '';
    }
  }

  async function deleteSelectedBrokenShortcuts() {
    if (selectedBrokenShortcuts.length === 0) return;
    confirmDeletion(selectedBrokenShortcuts);
  }

  function handleFileSelection(file: string, type: 'temp' | 'large' | 'duplicate' | 'empty' | 'broken_shortcut') {
    switch (type) {
      case 'temp':
        if (selectedTempFiles.includes(file)) {
          selectedTempFiles = selectedTempFiles.filter(f => f !== file);
        } else {
          selectedTempFiles = [...selectedTempFiles, file];
        }
        break;
      case 'large':
        if (selectedLargeFiles.includes(file)) {
          selectedLargeFiles = selectedLargeFiles.filter(f => f !== file);
        } else {
          selectedLargeFiles = [...selectedLargeFiles, file];
        }
        break;
      case 'duplicate':
        if (selectedDuplicateFiles.includes(file)) {
          selectedDuplicateFiles = selectedDuplicateFiles.filter(f => f !== file);
        } else {
          selectedDuplicateFiles = [...selectedDuplicateFiles, file];
        }
        break;
      case 'empty':
        if (selectedEmptyFolders.includes(file)) {
          selectedEmptyFolders = selectedEmptyFolders.filter(f => f !== file);
        } else {
          selectedEmptyFolders = [...selectedEmptyFolders, file];
        }
        break;
      case 'broken_shortcut':
        if (selectedBrokenShortcuts.includes(file)) {
          selectedBrokenShortcuts = selectedBrokenShortcuts.filter(f => f !== file);
        } else {
          selectedBrokenShortcuts = [...selectedBrokenShortcuts, file];
        }
        break;
    }
  }

  function toggleSelectAll(type: 'temp' | 'large' | 'duplicate' | 'empty' | 'broken_shortcut') {
    switch (type) {
      case 'temp':
        if (selectedTempFiles.length === tempFiles.length) {
          selectedTempFiles = [];
        } else {
          selectedTempFiles = tempFiles.map(f => f.path);
        }
        break;
      case 'large':
        if (selectedLargeFiles.length === largeFiles.length) {
          selectedLargeFiles = [];
        } else {
          selectedLargeFiles = largeFiles.map(f => f.path);
        }
        break;
      case 'duplicate':
        if (selectedDuplicateFiles.length === duplicateFiles.length) {
          selectedDuplicateFiles = [];
        } else {
          selectedDuplicateFiles = duplicateFiles.map(f => f.path);
        }
        break;
      case 'empty':
        if (selectedEmptyFolders.length === emptyFolders.length) {
          selectedEmptyFolders = [];
        } else {
          selectedEmptyFolders = emptyFolders.map(f => f.path);
        }
        break;
      case 'broken_shortcut':
        if (selectedBrokenShortcuts.length === brokenShortcuts.length) {
          selectedBrokenShortcuts = [];
        } else {
          selectedBrokenShortcuts = brokenShortcuts.map(f => f.path);
        }
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
      message = `Deleting ${filesToDelete.length} files...`;
      const deletedCount: number = await invoke('delete_files', { files: filesToDelete });
      message = `Successfully deleted ${deletedCount} files.`;

      tempFiles = tempFiles.filter(f => !filesToDelete.includes(f.path));
      largeFiles = largeFiles.filter(f => !filesToDelete.includes(f.path));
      duplicateFiles = duplicateFiles.filter(f => !filesToDelete.includes(f.path));
      emptyFolders = emptyFolders.filter(f => !filesToDelete.includes(f.path));
      brokenShortcuts = brokenShortcuts.filter(f => !filesToDelete.includes(f.path));

      selectedTempFiles = [];
      selectedLargeFiles = [];
      selectedDuplicateFiles = [];
      selectedEmptyFolders = [];
      selectedBrokenShortcuts = [];
      filesToDelete = [];

    } catch (error) {
      message = `Error deleting files: ${error}`;
      console.error(error);
    } finally {
      isLoading = false;
    }
  }

  function cancelDeletion() {
    showConfirmationModal = false;
    filesToDelete = [];
  }
 </script>

<style>
  :global(:root) {
    --background: hsl(220, 13%, 9%);
    --foreground: hsl(220, 9%, 85%);
    --card: hsl(220, 13%, 12%);
    --card-foreground: hsl(220, 9%, 85%);
    --primary: hsl(268, 100%, 70%);
    --primary-foreground: hsl(220, 13%, 9%);
    --secondary: hsl(220, 13%, 15%);
    --secondary-foreground: hsl(220, 9%, 85%);
    --muted: hsl(220, 13%, 15%);
    --muted-foreground: hsl(220, 9%, 46%);
    --success: hsl(142, 76%, 36%);
    --success-foreground: hsl(220, 9%, 85%);
    --border: hsl(220, 13%, 18%);
    --avelonia-purple: hsl(268, 100%, 70%);
    --avelonia-dark: hsl(220, 13%, 9%);
    --avelonia-darker: hsl(220, 13%, 6%);
    --avelonia-card: hsl(220, 13%, 12%);
    --avelonia-border: hsl(220, 13%, 18%);
    --avelonia-text: hsl(220, 9%, 85%);
    --avelonia-text-muted: hsl(220, 9%, 46%);
    --avelonia-success: hsl(142, 76%, 36%);
    --avelonia-blue: hsl(212, 100%, 50%);
    --avelonia-warning: hsl(45, 100%, 50%);
    --avelonia-danger: hsl(0, 84%, 60%);
    --gradient-purple: linear-gradient(135deg, hsl(268, 100%, 70%) 0%, hsl(268, 100%, 60%) 100%);
    --gradient-card: linear-gradient(to bottom right, #121212 0%, #16171A 43%, #121316 100%);
    --shadow-purple: 0 10px 30px -10px hsl(268, 100%, 70%, 0.3);
    --shadow-card: 0 4px 6px -1px hsl(220, 13%, 6%, 0.4);
    --transition-smooth: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .panel {
    background-color: #121212;
    background-image: var(--gradient-card);
    border: 1px solid var(--avelonia-border);
    border-radius: 0.75rem;
    padding: 1.5rem;
    box-shadow: var(--shadow-card);
    transition: var(--transition-smooth);
  }

  .panel:hover {
    transform: translateY(-2px);
    box-shadow: var(--shadow-card), var(--shadow-purple);
  }

  .panel h2 {
    color: var(--avelonia-text);
  }

  .panel p,
  .panel h3,
  .panel span,
  .panel li {
    color: var(--avelonia-text);
  }

  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.65rem 0.95rem;
    border-radius: 0.65rem;
    font-weight: 600;
    letter-spacing: 0.01em;
    border: 1px solid transparent;
    cursor: pointer;
    user-select: none;
    transition: transform 0.15s ease, box-shadow 0.2s ease, background-color 0.2s ease, border-color 0.2s ease;
    box-shadow: 0 1px 1px rgba(0,0,0,0.12), 0 2px 8px rgba(0,0,0,0.25);
    will-change: transform;
  }

  .btn:hover { transform: translateY(-1px); }
  .btn:active { transform: translateY(0); }
  .btn:disabled { opacity: 0.6; cursor: not-allowed; transform: none; }
  .btn:focus-visible { outline: none; box-shadow: 0 0 0 3px hsl(268 100% 70% / 0.35), 0 1px 1px rgba(0,0,0,0.12), 0 2px 8px rgba(0,0,0,0.25); }

  .btn-primary {
    background-image: var(--gradient-purple);
    color: white;
    border-color: hsl(268 100% 70% / 0.35);
    box-shadow: 0 8px 24px -8px hsl(268 100% 70% / 0.35);
  }

  .btn-secondary {
    background-color: var(--secondary);
    color: var(--avelonia-text);
    border-color: var(--avelonia-border);
  }

  .btn-danger {
    background-image: linear-gradient(135deg, hsl(0 84% 60%) 0%, hsl(0 84% 55%) 100%);
    color: white;
    border-color: hsl(0 84% 60% / 0.35);
    box-shadow: 0 8px 24px -8px hsl(0 84% 60% / 0.35);
  }
</style>

<div class="p-4">
  <h1 class="text-2xl font-bold mb-4">PC Cleaner & Cloner</h1>

  <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
    <div class="panel">
      <h2 class="text-xl font-semibold mb-4">Cleaner</h2>

      <div class="mb-6">
        <h3 class="text-lg font-medium mb-2">Temporary Files</h3>
        <button
          on:click={getTempFiles}
          class="btn btn-primary mr-2"
          disabled={isLoading}
        >
          Scan Temp Files
        </button>
        <button
          on:click={getTempFiles}
          class="btn btn-secondary"
          disabled={isLoading}
        >
          Refresh
        </button>
        <button
          on:click={deleteSelectedTempFiles}
          class="btn btn-danger"
          disabled={selectedTempFiles.length === 0 || isLoading}
        >
          Clean Selected Temp Files ({selectedTempFiles.length})
        </button>
        {#if tempFiles.length > 0}
          <div class="mt-4">
            <label class="inline-flex items-center">
              <input
                type="checkbox"
                class="h-4 w-4 rounded border-gray-600 bg-gray-800 accent-blue-600"
                on:change={() => toggleSelectAll('temp')}
                checked={selectedTempFiles.length === tempFiles.length && tempFiles.length > 0}
              />
              <span class="ml-2 text-gray-300">Select All</span>
            </label>
            <ul class="mt-2 text-sm text-gray-400 max-h-48 overflow-y-auto border border-gray-700 p-2 rounded">
              {#each tempFiles as file (file.path)}
                <li class="flex items-center">
                  <input
                    type="checkbox"
                    class="h-4 w-4 rounded border-gray-600 bg-gray-800 accent-blue-600"
                    checked={selectedTempFiles.includes(file.path)}
                    on:change={() => handleFileSelection(file.path, 'temp')}
                  />
                  <span class="ml-2">{file.path}</span>
                </li>
              {/each}
            </ul>
          </div>
        {/if}
      </div>

      <div class="mb-6">
        <h3 class="text-lg font-medium mb-2">Recycle Bin</h3>
        <button
          on:click={confirmEmptyRecycleBin}
          class="btn btn-danger"
          disabled={isLoading}
        >
          Empty Recycle Bin
        </button>
      </div>

      <div class="mb-6">
        <h3 class="text-lg font-medium mb-2">Disk Space</h3>
        <p class="text-gray-300">Total: {formatBytes(totalDiskSpace)}</p>
        <p class="text-gray-300">Available: {formatBytes(availableDiskSpace)}</p>
        <button
          on:click={getDiskInfo}
          class="btn btn-primary mt-2"
          disabled={isLoading}
        >
          Refresh Disk Info
        </button>
      </div>

      <div class="mb-6">
        <h3 class="text-lg font-medium mb-2">Large Files (>{formatBytes(100 * 1024 * 1024)})</h3>
        <button
          on:click={findLargeFiles}
          class="btn btn-primary mr-2"
          disabled={isLoading}
        >
          Find Large Files
        </button>
        <button
          on:click={findLargeFiles}
          class="btn btn-secondary"
          disabled={isLoading}
        >
          Refresh
        </button>
        <button
          on:click={deleteSelectedLargeFiles}
          class="btn btn-danger"
          disabled={selectedLargeFiles.length === 0 || isLoading}
        >
          Delete Selected Large Files ({selectedLargeFiles.length})
        </button>
        {#if largeFiles.length > 0}
          <div class="mt-4">
            <label class="inline-flex items-center">
              <input
                type="checkbox"
                class="h-4 w-4 rounded border-gray-600 bg-gray-800 accent-blue-600"
                on:change={() => toggleSelectAll('large')}
                checked={selectedLargeFiles.length === largeFiles.length && largeFiles.length > 0}
              />
              <span class="ml-2 text-gray-300">Select All</span>
            </label>
            <ul class="mt-2 text-sm text-gray-400 max-h-48 overflow-y-auto border border-gray-700 p-2 rounded">
              {#each largeFiles as file (file.path)}
                <li class="flex items-center justify-between">
                  <label class="inline-flex items-center">
                    <input
                      type="checkbox"
                      class="h-4 w-4 rounded border-gray-600 bg-gray-800 accent-blue-600"
                      checked={selectedLargeFiles.includes(file.path)}
                      on:change={() => handleFileSelection(file.path, 'large')}
                    />
                    <span class="ml-2">{file.path}</span>
                  </label>
                  {#if file.size}
                    <span class="text-xs text-gray-500">{formatBytes(file.size)}</span>
                  {/if}
                </li>
              {/each}
            </ul>
          </div>
        {/if}
      </div>

      <div class="mb-6">
        <h3 class="text-lg font-medium mb-2">Duplicate Files</h3>
        <button
          on:click={findDuplicateFiles}
          class="btn btn-primary mr-2"
          disabled={isLoading}
        >
          Find Duplicate Files
        </button>
        <button
          on:click={findDuplicateFiles}
          class="btn btn-secondary"
          disabled={isLoading}
        >
          Refresh
        </button>
        <button
          on:click={deleteSelectedDuplicateFiles}
          class="btn btn-danger"
          disabled={selectedDuplicateFiles.length === 0 || isLoading}
        >
          Delete Selected Duplicate Files ({selectedDuplicateFiles.length})
        </button>
        {#if duplicateFiles.length > 0}
          <div class="mt-4">
            <label class="inline-flex items-center">
              <input
                type="checkbox"
                class="h-4 w-4 rounded border-gray-600 bg-gray-800 accent-blue-600"
                on:change={() => toggleSelectAll('duplicate')}
                checked={selectedDuplicateFiles.length === duplicateFiles.length && duplicateFiles.length > 0}
              />
              <span class="ml-2 text-gray-300">Select All</span>
            </label>
            <ul class="mt-2 text-sm text-gray-400 max-h-48 overflow-y-auto border border-gray-700 p-2 rounded">
              {#each duplicateFiles as file (file.path)}
                <li class="flex items-center">
                  <input
                    type="checkbox"
                    class="h-4 w-4 rounded border-gray-600 bg-gray-800 accent-blue-600"
                    checked={selectedDuplicateFiles.includes(file.path)}
                    on:change={() => handleFileSelection(file.path, 'duplicate')}
                  />
                  <span class="ml-2">{file.path}</span>
                </li>
              {/each}
            </ul>
          </div>
        {/if}
      </div>

      <div class="mb-6">
        <h3 class="text-lg font-medium mb-2">Empty Folders</h3>
        <button
          on:click={findEmptyFolders}
          class="btn btn-primary mr-2"
          disabled={isLoading}
        >
          Find Empty Folders
        </button>
        <button
          on:click={findEmptyFolders}
          class="btn btn-secondary"
          disabled={isLoading}
        >
          Refresh
        </button>
        <button
          on:click={deleteSelectedEmptyFolders}
          class="btn btn-danger"
          disabled={selectedEmptyFolders.length === 0 || isLoading}
        >
          Delete Selected Empty Folders ({selectedEmptyFolders.length})
        </button>
        {#if emptyFolders.length > 0}
          <div class="mt-4">
            <label class="inline-flex items-center">
              <input
                type="checkbox"
                class="h-4 w-4 rounded border-gray-600 bg-gray-800 accent-blue-600"
                on:change={() => toggleSelectAll('empty')}
                checked={selectedEmptyFolders.length === emptyFolders.length && emptyFolders.length > 0}
              />
              <span class="ml-2 text-gray-300">Select All</span>
            </label>
            <ul class="mt-2 text-sm text-gray-400 max-h-48 overflow-y-auto border border-gray-700 p-2 rounded">
              {#each emptyFolders as folder (folder.path)}
                <li class="flex items-center">
                  <input
                    type="checkbox"
                    class="h-4 w-4 rounded border-gray-600 bg-gray-800 accent-blue-600"
                    checked={selectedEmptyFolders.includes(folder.path)}
                    on:change={() => handleFileSelection(folder.path, 'empty')}
                  />
                  <span class="ml-2">{folder.path}</span>
                </li>
              {/each}
            </ul>
          </div>
        {/if}
      </div>

      <div class="mb-6">
        <h3 class="text-lg font-medium mb-2">Broken Shortcuts</h3>
        <button
          on:click={findBrokenShortcuts}
          class="btn btn-primary mr-2"
          disabled={isLoading}
        >
          Find Broken Shortcuts
        </button>
        <button
          on:click={findBrokenShortcuts}
          class="btn btn-secondary"
          disabled={isLoading}
        >
          Refresh
        </button>
        <button
          on:click={deleteSelectedBrokenShortcuts}
          class="btn btn-danger"
          disabled={selectedBrokenShortcuts.length === 0 || isLoading}
        >
          Delete Selected Broken Shortcuts ({selectedBrokenShortcuts.length})
        </button>
        {#if brokenShortcuts.length > 0}
          <div class="mt-4">
            <label class="inline-flex items-center">
              <input
                type="checkbox"
                class="h-4 w-4 rounded border-gray-600 bg-gray-800 accent-blue-600"
                on:change={() => toggleSelectAll('broken_shortcut')}
                checked={selectedBrokenShortcuts.length === brokenShortcuts.length && brokenShortcuts.length > 0}
              />
              <span class="ml-2 text-gray-300">Select All</span>
            </label>
            <ul class="mt-2 text-sm text-gray-400 max-h-48 overflow-y-auto border border-gray-700 p-2 rounded">
              {#each brokenShortcuts as shortcut (shortcut.path)}
                <li class="flex items-center">
                  <input
                    type="checkbox"
                    class="h-4 w-4 rounded border-gray-600 bg-gray-800 accent-blue-600"
                    checked={selectedBrokenShortcuts.includes(shortcut.path)}
                    on:change={() => handleFileSelection(shortcut.path, 'broken_shortcut')}
                  />
                  <span class="ml-2">{shortcut.path}</span>
                </li>
              {/each}
            </ul>
          </div>
        {/if}
      </div>
    </div>

    <div class="panel">
      <h2 class="text-xl font-semibold mb-4">Cloner / Backup</h2>
      <p class="text-gray-400">
        Features for file synchronization and backup will be implemented here.
      </p>
      <div class="mt-4">
        <h3 class="text-lg font-medium mb-2">File Synchronization</h3>
        <button
          class="btn btn-primary"
          disabled
        >
          Sync Folders (Coming Soon)
        </button>
      </div>
    </div>
  </div>

  {#if message || progressMessage}
    <p class="mt-4 text-center text-gray-300">
      {#if isLoading}
        <span class="animate-pulse">Loading...</span>
      {/if}
      {progressMessage || message}
    </p>
  {/if}

  {#if showConfirmationModal}
    <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div class="panel max-w-md w-full">
        <h2 class="text-xl font-bold mb-4 text-white">Confirm Deletion</h2>
        <p class="text-gray-300 mb-4">
          Are you sure you want to delete {filesToDelete.length} selected file(s)? This action cannot be undone.
        </p>
        <div class="flex justify-end space-x-4">
          <button
            on:click={cancelDeletion}
            class="btn btn-secondary"
          >
            Cancel
          </button>
          <button
            on:click={executeDeletion}
            class="btn btn-danger"
          >
            Delete
          </button>
        </div>
      </div>
    </div>
  {/if}

  {#if showRecycleBinConfirmationModal}
    <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div class="panel max-w-md w-full">
        <h2 class="text-xl font-bold mb-4 text-white">Confirm Empty Recycle Bin</h2>
        <p class="text-gray-300 mb-4">
          Are you sure you want to empty the recycle bin? This action cannot be undone.
        </p>
        <div class="flex justify-end space-x-4">
          <button
            on:click={() => showRecycleBinConfirmationModal = false}
            class="btn btn-secondary"
          >
            Cancel
          </button>
          <button
            on:click={emptyRecycleBin}
            class="btn btn-danger"
          >
            Empty
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>
