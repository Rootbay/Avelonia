import { BUILT_IN_DOWNLOAD_GROUPS } from './builtInDownloadsData';

export const BUILT_IN_DOWNLOADS = BUILT_IN_DOWNLOAD_GROUPS.flatMap((group) => group.downloads);
export const BUILT_IN_DOWNLOAD_GROUPS_LIST = BUILT_IN_DOWNLOAD_GROUPS;
