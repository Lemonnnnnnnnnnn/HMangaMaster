<template>
    <div class="flex flex-col gap-8 p-8 text-white">
        <div class="flex flex-col gap-4">
            <div class="text-xl">下载目录</div>
            <div class="flex gap-4">
                <Input class="flex-1 cursor-pointer" v-model="downloadDir" placeholder="请选择下载目录"
                    @click="changeOutputDir" />
            </div>
        </div>

        <div class="flex flex-col gap-4">
            <div class="text-xl">漫画库</div>
            <div class="flex gap-2 ">
                <div v-for="library in libraries" :key="library" class="flex items-center gap-2">
                    <button :class="{ 'bg-neutral-500/50': library === activeLibrary }"
                        class=" cursor-pointer hover:bg-neutral-500/50 rounded-2xl border-1 border-neutral-300/50 py-2 px-4"
                        @click="changeActiveLibrary(library)">{{ library }}</button>
                </div>
            </div>
            <div class="flex justify-end">
                <button class="rounded-2xl border-1 border-neutral-300/50 py-2 px-4" @click="addLibrary">添加漫画库</button>
            </div>
        </div>

        <div class="flex flex-col gap-4">
            <div class="text-xl">代理设置</div>
            <div class="flex gap-4">
                <Input @blur="saveProxy" class="flex-1" v-model="proxyUrl" placeholder="请输入代理地址" />
            </div>
        </div>

        <div class="flex flex-col gap-4">
            <div class="text-xl">Pixiv 设置</div>
            <div class="flex gap-4">
                <Input @blur="savePixivConfig" class="flex-1" v-model="pixivCookies" placeholder="请输入 Pixiv cookies" />
            </div>
        </div>

        <div class="flex flex-col gap-4">
            <div class="text-xl">日志</div>
            <div class="flex flex-col gap-2 text-neutral-300/90">
                <div>目录：<span class="select-all">{{ logInfo?.dir || '-' }}</span></div>
                <div>配置文件：<span class="select-all">{{ configPath || '-' }}</span></div>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { Input } from '@/components';
import { onMounted, ref, watch, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'vue-sonner';
import { debounce } from '@/utils';

let proxyUrl = ref("")
let downloadDir = ref("")
let libraries = ref<string[]>([])
let activeLibrary = ref("")
let logInfo = ref<any>(null)
let pixivConfig = ref<any>(null)
let configPath = ref("")

const pixivCookies = computed({
    get() {
        return pixivConfig.value?.auth?.cookies || ''
    },
    set(value: string) {
        pixivConfig.value.auth.cookies = value
    }
})

const saveProxy = debounce((e: Event) => {
    invoke('config_set_proxy', { proxy: (e.target as HTMLInputElement).value }).then(() => {
        refreshConfig()
    })
}, 1000)

const savePixivConfig = debounce(async () => {
    // 如果没有 pixivConfig，创建一个默认的
    if (!pixivConfig.value) {
        pixivConfig.value = {
            base: { concurrency: 3 },
            auth: { cookies: '' },
            site_specific: null
        }
    }

    await invoke('config_set_parser_config', {
        parserName: 'pixiv',
        config: pixivConfig.value
    });
    refreshConfig();
}, 1000)


async function refreshConfig() {
    proxyUrl.value = await invoke<string>('config_get_proxy');
    downloadDir.value = await invoke<string>('config_get_output_dir');
    libraries.value = await invoke<string[]>('config_get_libraries');
    activeLibrary.value = await invoke<string>('config_get_active_library');
    configPath.value = await invoke<string>('config_get_config_path');

    try {
        pixivConfig.value = await invoke<any>('config_get_parser_config', { parserName: 'pixiv' });
    } catch (e) {
        pixivConfig.value = null;
    }

    try {
        logInfo.value = await invoke<any>('logger_get_info');
    } catch (e) {
        logInfo.value = null
    }
}

async function changeOutputDir() {
    invoke('config_set_output_dir').then(() => {
        toast.success("设置成功！")
        refreshConfig()
    })
}

async function changeActiveLibrary(library: string) {
    invoke('config_set_active_library', { library }).then(() => {
        toast.success("设置成功！")
        refreshConfig()
    })
}

async function addLibrary() {
    invoke('config_add_library').then(() => {
        toast.success("添加成功！")
        refreshConfig().then(() => {
            invoke('library_load', { path: activeLibrary.value }).then(() => {
                toast.success("加载成功！")
            })
        })
    })
}

onMounted(async () => {
    refreshConfig()
})

</script>

<style scoped></style>