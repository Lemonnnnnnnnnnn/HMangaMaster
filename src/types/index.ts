export type DownloadTaskDTO = {
    id: string;
    url: string;
    status: string;
    save_path: string;
    start_time: string;
    complete_time: string;
    updated_at: string;
    error: string;
    name: string;
    progress: Progress;
}

type Progress = {
    current: number;
    total: number;
}
