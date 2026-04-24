export type DownloadTaskDTO = {
    id: string;
    url: string;
    status: string;
    save_path: string;
    start_time: string;
    complete_time: string;
    updated_at: string;
    error: string;
    failedCount?: number;
    failedFiles?: FailedFile[];
    name: string;
    progress: Progress;
}

export type FailedFile = {
    index: number;
    url: string;
    path: string;
    error: string;
}

type Progress = {
    current: number;
    total: number;
}
