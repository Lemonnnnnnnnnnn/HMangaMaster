<script setup lang="ts">
// import routes from './routes';
import { Toaster, toast } from 'vue-sonner';
import { onMounted } from 'vue';
import { listen } from '@tauri-apps/api/event';
import { SideBar } from './components';
import 'vue-sonner/style.css'

onMounted(() => {
  listen('download:completed', (e) => {
    const data: any = e.payload;
    toast.success('下载完成！', { description: typeof data === 'string' ? data : (data?.taskId || ''), duration: 5000 });
  });
  listen('download:failed', (e) => {
    const data: any = e.payload;
    toast.error('下载失败', { description: data?.message || '下载过程中发生错误', duration: 5000 });
  });
  listen('download:cancelled', (e) => {
    const data: any = e.payload;
    toast.warning('下载已取消', { description: data?.taskId || '', duration: 3000 });
  });
});

</script>

<template>

  <div class="flex h-screen bg-neutral-900">
    <SideBar />
    <main class="flex-1 h-screen overflow-auto">
      <RouterView />
    </main>
  </div>
  <Toaster />
</template>

<style scoped></style>
