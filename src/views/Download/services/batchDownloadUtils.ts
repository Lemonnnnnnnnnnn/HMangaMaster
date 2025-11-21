import { invoke } from '@tauri-apps/api/core';
import { toast } from 'vue-sonner';
import { validateUrl } from './downloadUtils';

/**
 * 批量下载结果接口
 */
export interface BatchDownloadResult {
  success: boolean;
  taskIds?: string[];
  error?: string;
  extractedCount?: number;
}

/**
 * 批量下载选项接口
 */
export interface BatchDownloadOptions {
  url: string;
  onStart?: () => void;
  onError?: (error: string) => void;
  onSuccess?: (taskIds: string[], extractedCount: number) => void;
}

/**
 * 执行批量下载任务
 * @param options 批量下载选项
 * @returns Promise<BatchDownloadResult>
 */
export async function executeBatchDownload(options: BatchDownloadOptions): Promise<BatchDownloadResult> {
  const { url, onStart, onError, onSuccess } = options;

  // 验证URL
  if (!validateUrl(url)) {
    const error = '请输入有效的网址';
    onError?.(error);
    return { success: false, error };
  }

  try {
    // 调用下载开始回调
    onStart?.();

    // 执行批量爬取
    const taskIds = await invoke<string[]>('batch_start_crawl', { url: url.trim() });
    console.log('Batch download task IDs:', taskIds);

    if (taskIds && taskIds.length > 0) {
      // 批量下载任务创建成功
      onSuccess?.(taskIds, taskIds.length);
      return {
        success: true,
        taskIds,
        extractedCount: taskIds.length
      };
    } else {
      const error = '批量下载失败，未找到任何漫画链接';
      onError?.(error);
      return { success: false, error };
    }
  } catch (err: any) {
    const error = `批量下载出错: ${err.message || '未知错误'}`;
    onError?.(error);
    return { success: false, error };
  }
}

/**
 * 创建批量下载处理器
 * @param callbacks 回调函数集合
 * @returns 批量下载处理函数
 */
export function createBatchDownloadHandler(callbacks: {
  onStart?: () => void;
  onSuccess?: (taskIds: string[], url: string, extractedCount: number) => void;
  onError?: (error: string) => void;
  onFinally?: () => void;
}) {
  return async (url: string): Promise<BatchDownloadResult> => {
    const { onStart, onSuccess, onError, onFinally } = callbacks;

    try {
      onStart?.();
      const result = await executeBatchDownload({
        url,
        onError,
        onSuccess: (taskIds, extractedCount) => onSuccess?.(taskIds, url, extractedCount)
      });

      return result;
    } finally {
      onFinally?.();
    }
  };
}