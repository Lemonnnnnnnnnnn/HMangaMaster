import { invoke } from '@tauri-apps/api/core';
import type { Manga } from '../stores/homeStore';
import { useHomeStore } from '../stores/homeStore';

export class MangaService {
  private homeStore: ReturnType<typeof useHomeStore>;
  constructor() {
    this.homeStore = useHomeStore();
  }
  /**
   * 初始化数据加载
   */
  async initialize(): Promise<void> {
    this.homeStore.loading = true;
    await this.loadMangas();
    this.homeStore.loading = false;
  }

  /**
   * 加载漫画列表
   */
  async loadMangas(): Promise<void> {
    this.homeStore.loading = true;
    // 初始化数据加载
    await invoke('library_load_active');

    const mangasData = await invoke<Manga[]>('library_get_all_mangas');
    this.homeStore.mangas = mangasData;

    // 预加载每个漫画的预览图
    const imageCache = this.homeStore.mangaImages;
    for (let manga of mangasData) {
      if (!imageCache.has(manga.previewImg)) {
        const imageUrl = await invoke<string>('library_get_image_data_url', { path: manga.previewImg });
        imageCache.set(manga.previewImg, imageUrl);
      }
    }
    this.homeStore.mangaImages = imageCache;
    this.homeStore.loading = false;
  }

  /**
   * 删除漫画
   */
  async deleteManga(manga: Manga): Promise<boolean> {
    if (!confirm(`确定要删除 "${manga.name}" 吗？这将永久删除该文件夹及其内容！`)) {
      return false;
    }

    this.homeStore.loading = true;
    const success = await invoke<boolean>('library_delete_manga', { path: manga.path });

    if (success) {
      const currentMangas = this.homeStore.mangas;
      this.homeStore.mangas = currentMangas.filter((m) => m.path !== manga.path);
    } else {
      alert('删除失败！');
    }

    this.homeStore.loading = false;
    return success;
  }

  /**
   * 获取漫画预览图
   */
  getMangaImage(previewImg: string): string {
    const imageCache = this.homeStore.mangaImages;
    return imageCache.get(previewImg) || '';
  }
}