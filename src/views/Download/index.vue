<template>
    <div class="p-8 h-screen flex flex-col">
        <div class="flex gap-4 mt-2">
            <Input autofocus v-model="url" class="flex-1" help="please input the target manga url" @keydown="handleKeydown" />
            <Button @click="handleDownload">
                <div class="flex items-center gap-2">
                    <Download :size="16" class="text-white" />
                    <span>下载</span>
                </div>
            </Button>
        </div>

        <!-- 批量下载 -->
        <div class="mt-6">
            <div class="flex gap-4">
                <Input
                    v-model="batchUrl"
                    class="flex-1"
                    help="请输入包含多个漫画链接的页面URL（如搜索结果页、收藏夹等）"
                    @keydown="handleBatchKeydown"
                    placeholder="https://e-hentai.org/?page=1&f_search=tag"
                />
                <Button @click="handleBatchDownload" :disabled="isBatchDownloading">
                    <div class="flex items-center gap-2">
                        <Package :size="16" class="text-white" />
                        <span>{{ isBatchDownloading ? '批量处理中...' : '批量下载' }}</span>
                    </div>
                </Button>
            </div>

            <!-- 批量下载结果提示 -->
            <div v-if="batchResult" class="mt-2 p-3 rounded-md" :class="batchResult.success ? 'bg-green-50 text-green-800' : 'bg-red-50 text-red-800'">
                <div class="text-sm">
                    {{ batchResult.success
                        ? `批量下载任务创建成功！共提取到 ${batchResult.extractedCount} 个漫画下载任务`
                        : batchResult.error
                    }}
                </div>
            </div>
        </div>

        <div class="h-1 border-b border-neutral-300/50 w-full my-8"></div>

        <div class="flex items-center justify-end gap-2">
            <Button @click="router.push('/history')">
                <div class="flex items-center gap-2">
                    <History :size="16" class="text-white" />
                    <span>历史记录</span>
                </div>
            </Button>
        </div>
        <div class="flex-1 overflow-auto">
            <TaskList
                class="mt-2"
                :tasks="activeTasks"
                mode="active"
                :retrying-tasks="retryingTasks"
                @cancel="onCancelTask"
                @retry="onRetryTask"
            />
        </div>
    </div>

</template>

<script setup lang="ts">
import { Button, Input, TaskList } from '@/components';
import { Download, History, Package } from 'lucide-vue-next';
import { storeToRefs } from 'pinia';
import { onMounted, onUnmounted, ref } from 'vue';
import { toast } from 'vue-sonner';
import { createDownloadHandler, createBatchDownloadHandler } from './services';
import { useDownloadStore } from './stores';
import { useRouter } from 'vue-router';

const router = useRouter();

const downloadStore = useDownloadStore();

let url = ref('');
let batchUrl = ref('');
let isBatchDownloading = ref(false);
let batchResult = ref<any>(null);

const { activeTasks, retryingTasks } = storeToRefs(downloadStore);

function handleKeydown(event: any) {
    if (event.key === 'Enter') {
        handleDownload();
    }
}

function handleBatchKeydown(event: any) {
    if (event.key === 'Enter') {
        handleBatchDownload();
    }
}

onMounted(async () => {
    await downloadStore.initializeStore();
    console.log("组件已挂载，轮询已开始");
});

onUnmounted(() => {
    downloadStore.stopPolling();
    console.log("组件已销毁，轮询已停止");
});

// 处理下载
async function handleDownload() {
    if (!url.value.trim()) {
        toast.error('请输入网址');
        return;
    }

    await downloadHandler(url.value.trim());
    // url.value = '';
}

// 处理批量下载
async function handleBatchDownload() {
    if (!batchUrl.value.trim()) {
        toast.error('请输入批量下载网址');
        return;
    }

    if (isBatchDownloading.value) {
        toast.warning('批量下载正在进行中，请稍候...');
        return;
    }

    await batchDownloadHandler(batchUrl.value.trim());
    batchResult.value = null; // 清空之前的结果
}

// 创建下载处理器
const downloadHandler = createDownloadHandler({
    onStart() {
        url.value = '';
    },
    onError: (errorMsg) => {
        toast.error(errorMsg);
    },
});

// 创建批量下载处理器
const batchDownloadHandler = createBatchDownloadHandler({
    onStart() {
        isBatchDownloading.value = true;
        batchResult.value = null;
        batchUrl.value = '';
        toast.info('开始解析批量下载链接...');
    },
    onSuccess: (taskIds: string[], url: string, extractedCount: number) => {
        isBatchDownloading.value = false;
        batchResult.value = {
            success: true,
            taskIds,
            extractedCount
        };
        toast.success(`批量下载任务创建成功！共创建 ${extractedCount} 个下载任务`);
    },
    onError: (errorMsg: string) => {
        isBatchDownloading.value = false;
        batchResult.value = {
            success: false,
            error: errorMsg
        };
        toast.error(errorMsg);
    },
    onFinally() {
        isBatchDownloading.value = false;
    }
});

async function onCancelTask(taskId: string) {
    await downloadStore.cancelTask(taskId);
}

async function onRetryTask(taskId: string, retryType: 'full' | 'failedOnly') {
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

</script>

<style scoped></style>
