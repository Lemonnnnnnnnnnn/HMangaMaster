<template>

    <div v-if="tasks.length === 0">
        <div class="text-center h-80 flex items-center justify-center text-neutral-100">
            <p> no task</p>
        </div>
    </div>
    <table v-else class="w-full text-neutral-100 scroll-auto">
        <thead>
            <tr class="border-b border-neutral-300">
                <th>名字</th>
                <th v-if="mode === 'active'">url</th>
                <th>状态</th>
                <th v-if="mode === 'active'">进度</th>
                <th v-if="mode === 'history'">完成时间</th>
                <th v-if="mode === 'history'">失败原因</th>
                <th v-if="mode === 'history'">耗时</th>
                <th v-if="mode === 'active'">操作</th>
            </tr>
        </thead>

        <tbody>
            <tr v-for="task in tasks" :key="task.id" class="border-b border-neutral-500/50">
                <td :title="task.name" class="max-w-48">{{ task.name }}</td>
                <td v-if="mode === 'active'" :title="task.url" class="max-w-64">{{ task.url }}</td>
                <td>
                    <div class="flex items-center justify-center gap-2">
                        <component :is="getStatusIcon(task.status)?.icon" :size="16"
                            :class="getStatusIcon(task.status)?.class" />
                        <span>{{ formatStatus(task.status) }}</span>
                    </div>
                </td>
                <td v-if="mode === 'active'">
                    <div class="border border-neutral-300 rounded-xl h-2 w-full">
                        <div class="bg-neutral-300 rounded-xl h-full transition-all duration-300"
                            :style="{ width: `${calculateProgressPercentage(task.progress?.current ?? 0, task.progress?.total ?? 0)}%` }">
                        </div>
                    </div>
                </td>
                <td v-if="mode === 'history'">{{ task.status === 'completed' ? formatTime(task.completeTime ?? '') : '-'
                    }}</td>
                <td v-if="mode === 'history'" class="max-w-64">
                    <span v-if="(task.status === 'failed' || task.status === 'partial_failed') && task.error"
                        :title="task.error"
                        :class="task.status === 'partial_failed' ? 'text-yellow-400' : 'text-red-400'"
                        class="text-xs truncate">
                        {{ task.error }}
                    </span>
                    <span v-else class="text-neutral-500">-</span>
                </td>
                <td v-if="mode === 'history'">
                    <span v-if="task.startTime && task.completeTime">{{ calculateTimeDifference(task.startTime,
                        task.completeTime) }}</span>
                    <span v-else class="text-neutral-500">-</span>
                </td>
                <td class="text-center">
                    <div class="flex items-center justify-center gap-2">
                        <Button v-if="canCancel(task.status)" size="sm" :disabled="isRetrying(task.id)"
                            @click="$emit('cancel', task.id)">停止下载</Button>

                        <!-- 重试下拉菜单 -->
                        <DropDown v-if="canRetry(task)" v-model="openDropdowns[task.id]" position="bottom-right"
                            align="end" @select="(value) => handleRetry(task.id, value)">
                            <template #trigger>
                                <Button size="sm" type="default" :disabled="isRetrying(task.id)">
                                    <div class="flex gap-1 items-center">
                                        <RotateCcw :size="14" class="mr-1" v-if="!isRetrying(task.id)" />
                                        <Loader :size="14" class="mr-1 animate-spin" v-else />
                                        {{ getRetryButtonText(task) }}
                                        <ChevronDown :size="14" class="ml-1" />
                                    </div>

                                </Button>
                            </template>

                            <button
                                class="w-full text-left px-3 py-2 text-sm text-neutral-100 hover:bg-neutral-700 rounded-t-lg transition-colors duration-150"
                                data-value="full">
                                完整重试
                            </button>
                            <button v-if="task.status === 'partial_failed'"
                                class="w-full text-left px-3 py-2 text-sm text-neutral-100 hover:bg-neutral-700 rounded-b-lg border-t border-neutral-600 transition-colors duration-150"
                                data-value="failedOnly">
                                重试失败文件
                            </button>
                        </DropDown>
                    </div>
                </td>
            </tr>
        </tbody>

    </table>

</template>

<script setup lang="ts">
import { Loader, ArrowBigDownDash, CircleCheck, CircleX, CircleOff, AlertTriangle, RotateCcw, ChevronDown } from 'lucide-vue-next';
import { ref } from 'vue';
import { toast } from 'vue-sonner';
import Button from './Button.vue';
import DropDown from './DropDown.vue';
import { useDownloadStore } from '@/views/Download/stores';
// 类型最小替代，避免依赖 wailsjs
type DownloadTaskLike = {
    id: string;
    url: string;
    status: string;
    savePath?: string;
    name?: string;
    error?: string;
    failedCount?: number;
    progress?: { current?: number; total?: number } | null;
    startTime?: string;
    completeTime?: string;
    retryCount?: number;
    maxRetries?: number;
    retryable?: boolean;
};

const props = defineProps<{
    tasks: DownloadTaskLike[],
    mode?: 'active' | 'history',
    retryingTasks?: Set<string>
}>();

const emit = defineEmits<{
    (e: 'cancel', taskId: string): void
}>();

const downloadStore = useDownloadStore();

// 下拉菜单状态管理
const openDropdowns = ref<Record<string, boolean>>({})

async function handleRetry(taskId: string, retryType: 'full' | 'failedOnly') {
    try {
        if (retryType === 'full') {
            await downloadStore.retryTask(taskId);
            toast.success('任务重试已启动');
        } else {
            await downloadStore.retryFailedFilesOnly(taskId);
            toast.success('失败文件重试已启动');
        }
    } catch (err) {
        console.error('重试任务失败:', err);
        toast.error('重试任务失败');
    }
}

function calculateProgressPercentage(current: number, total: number): number {
    if (total <= 0) return 0;
    return Math.round((current / total) * 100);
}

function calculateTimeDifference(startTime: string, endTime: string): string {
    const startTimeDate = new Date(startTime);
    const endTimeDate = new Date(endTime);
    const timeDifference = endTimeDate.getTime() - startTimeDate.getTime();
    const seconds = Math.max(0, Math.floor(timeDifference / 1000));
    return `${seconds}秒`;
}

function formatTime(timeStr: string): string {
    if (!timeStr) return '';
    const date = new Date(timeStr);
    return `${date.toLocaleDateString()} ${date.toLocaleTimeString()}`;
}

function getStatusIcon(status: string) {
    if (status === 'pending') {
        return { icon: Loader, class: 'animate-spin' };
    } else if (status === 'parsing') {
        return { icon: Loader, class: 'animate-spin' };
    } else if (status === 'queued') {
        return { icon: Loader, class: 'animate-pulse text-blue-400' };
    } else if (status === 'downloading') {
        return { icon: ArrowBigDownDash, class: 'animate-bounce' };
    } else if (status === 'completed') {
        return { icon: CircleCheck, class: '' };
    } else if (status === 'partial_failed') {
        return { icon: AlertTriangle, class: 'text-yellow-500' };
    } else if (status === 'failed') {
        return { icon: CircleX, class: '' };
    } else if (status === 'cancelled') {
        return { icon: CircleOff, class: '' };
    }
}

function canCancel(status: string): boolean {
    return status === 'pending' || status === 'parsing' || status === 'queued' || status === 'downloading';
}

function canRetry(task: DownloadTaskLike): boolean {
    return (task.status === 'failed' || task.status === 'partial_failed') &&
        (task.retryable !== false) &&
        ((task.retryCount ?? 0) < (task.maxRetries ?? 3));
}

function formatStatus(status: string): string {
    const statusMap: Record<string, string> = {
        'pending': '等待中',
        'parsing': '解析中',
        'queued': '排队中',
        'downloading': '下载中',
        'completed': '已完成',
        'partial_failed': '部分失败',
        'failed': '失败',
        'cancelled': '已取消'
    };
    return statusMap[status] || status;
}

function getRetryButtonText(task: DownloadTaskLike): string {
    const retryCount = task.retryCount ?? 0;
    const maxRetries = task.maxRetries ?? 3;
    return `重试 (${retryCount}/${maxRetries})`;
}

function isRetrying(taskId: string): boolean {
    return props.retryingTasks?.has(taskId) ?? false;
}

const mode = props.mode ?? 'active';

</script>

<style scoped>
@reference "tailwindcss";

td,
th,
tr {
    @apply text-center text-xs px-2 py-4;
}

tr {
    @apply hover:bg-neutral-800;
}

td {
    @apply truncate
}
</style>