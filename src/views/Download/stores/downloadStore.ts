import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

// 轮询相关
const POLL_INTERVAL = 1000;
let pollTimer: ReturnType<typeof setInterval> | null = null;

export type DownloadStore = ReturnType<typeof useDownloadStore>;

export const useDownloadStore = defineStore('downloadStore', {
  state: () => ({
    activeTasks: ref<any[]>([]),
    historyTasks: ref<any[]>([]),
    loading: false as boolean,
    retryingTasks: ref<Set<string>>(new Set())
  }),
  getters: {
    activeTasksCount: (state) => state.activeTasks.length
  },
  actions: {
    async initializeStore() {
      try {
        // 直接启动轮询，让第一次轮询在 1 秒后自然发生，避免阻塞
        this.startPolling();
      } catch (err) {
        console.error('初始化store失败:', err);
      }
    },
    async pollTasks() {
      try {
        const active = await invoke<any[]>('task_active');
        this.activeTasks = active;
      } catch (err) {
        console.error('轮询任务状态出错:', err);
      }
    },
    startPolling() {
      this.stopPolling();
      pollTimer = setInterval(this.pollTasks, POLL_INTERVAL);
    },
    stopPolling() {
      if (pollTimer) {
        clearInterval(pollTimer);
        pollTimer = null;
      }
    },
    async cancelTask(taskId: string) {
      try {
        await invoke<boolean>('task_cancel', { taskId });
        await this.pollTasks();
      } catch (err) {
        console.error('取消任务出错:', err);
        throw err;
      }
    },

    async retryTask(taskId: string) {
      if (this.retryingTasks.has(taskId)) return;

      this.retryingTasks.add(taskId);

      try {
        await invoke<void>('task_retry', { taskId });
      } catch (err) {
        console.error('重试任务出错:', err);
        throw err;
      } finally {
        this.retryingTasks.delete(taskId);
      }
    },

    async retryFailedFilesOnly(taskId: string) {
      if (this.retryingTasks.has(taskId)) return;

      this.retryingTasks.add(taskId);

      try {
        await invoke<void>('task_retry_failed_files_only', { taskId });
      } catch (err) {
        console.error('重试失败文件出错:', err);
        throw err;
      } finally {
        this.retryingTasks.delete(taskId);
      }
    },
  }
});
