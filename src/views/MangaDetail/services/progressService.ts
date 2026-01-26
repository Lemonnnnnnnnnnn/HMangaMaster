/**
 * 漫画浏览进度管理服务
 * 使用localStorage保存和恢复漫画的阅读进度
 */

const PROGRESS_KEY = "manga_reading_progress";

export interface MangaProgress {
  scrollPercentage: number; // 滚动百分比 (0-1)
  timestamp: number;
  totalImages: number;
  scrollPosition?: number; // 向后兼容：旧的像素位置
}

export interface ProgressData {
  [mangaPath: string]: MangaProgress;
}

export class ProgressService {
  constructor() {
    ProgressService.cleanupOldProgress();
  }
  /**
   * 保存漫画的浏览进度
   * @param mangaPath 漫画路径，作为唯一标识符
   * @param scrollPercentage 滚动百分比 (0-1)
   * @param totalImages 总图片数量
   */
  static saveProgress(
    mangaPath: string,
    scrollPercentage: number,
    totalImages: number,
  ): void {
    try {
      const progressData = ProgressService.getAllProgress();

      progressData[mangaPath] = {
        scrollPercentage,
        timestamp: Date.now(),
        totalImages,
      };

      localStorage.setItem(PROGRESS_KEY, JSON.stringify(progressData));
    } catch (error) {
      console.error("保存阅读进度失败:", error);
    }
  }

  /**
   * 获取漫画的浏览进度
   * @param mangaPath 漫画路径
   * @returns 进度信息或null
   */
  static getProgress(mangaPath: string): MangaProgress | null {
    try {
      const progressData = ProgressService.getAllProgress();
      const progress = progressData[mangaPath];

      if (!progress) return null;

      // 向后兼容：如果是旧版本的数据，需要迁移
      if (
        progress.scrollPosition !== undefined &&
        progress.scrollPercentage === undefined
      ) {
        // 这是旧版本数据，需要在调用方处理迁移
        return {
          ...progress,
          scrollPercentage: 0, // 临时值，需要在scrollService中处理
        };
      }

      return progress;
    } catch (error) {
      console.error("读取阅读进度失败:", error);
      return null;
    }
  }

  /**
   * 删除指定漫画的进度记录
   * @param mangaPath 漫画路径
   */
  static removeProgress(mangaPath: string): void {
    try {
      const progressData = ProgressService.getAllProgress();
      delete progressData[mangaPath];
      localStorage.setItem(PROGRESS_KEY, JSON.stringify(progressData));
    } catch (error) {
      console.error("删除阅读进度失败:", error);
    }
  }

  /**
   * 清理过期的进度记录（超过30天）
   */
  static cleanupOldProgress(): void {
    try {
      const progressData = ProgressService.getAllProgress();
      const thirtyDaysAgo = Date.now() - 30 * 24 * 60 * 60 * 1000;

      Object.keys(progressData).forEach((mangaPath) => {
        if (progressData[mangaPath].timestamp < thirtyDaysAgo) {
          delete progressData[mangaPath];
        }
      });

      localStorage.setItem(PROGRESS_KEY, JSON.stringify(progressData));
    } catch (error) {
      console.error("清理过期进度失败:", error);
    }
  }

  /**
   * 获取所有进度数据
   * @returns 所有进度数据
   */
  private static getAllProgress(): ProgressData {
    try {
      const data = localStorage.getItem(PROGRESS_KEY);
      return data ? JSON.parse(data) : {};
    } catch (error) {
      console.error("解析进度数据失败:", error);
      return {};
    }
  }

  /**
   * 检查是否有保存的进度
   * @param mangaPath 漫画路径
   * @returns 是否存在进度记录
   */
  static hasProgress(mangaPath: string): boolean {
    const progress = ProgressService.getProgress(mangaPath);
    return (
      progress !== null &&
      (progress.scrollPercentage > 0 || progress.scrollPosition !== undefined)
    );
  }

  /**
   * 计算滚动百分比
   * @param scrollTop 当前滚动位置
   * @param scrollHeight 总滚动高度
   * @param clientHeight 可视区域高度
   * @returns 滚动百分比 (0-1)
   */
  static calculateScrollPercentage(
    scrollTop: number,
    scrollHeight: number,
    clientHeight: number,
  ): number {
    const maxScroll = scrollHeight - clientHeight;
    if (maxScroll <= 0) return 0;
    return Math.min(Math.max(scrollTop / maxScroll, 0), 1);
  }

  /**
   * 根据百分比计算实际滚动位置
   * @param scrollPercentage 滚动百分比 (0-1)
   * @param scrollHeight 总滚动高度
   * @param clientHeight 可视区域高度
   * @returns 实际滚动位置
   */
  static calculateScrollPosition(
    scrollPercentage: number,
    scrollHeight: number,
    clientHeight: number,
  ): number {
    const maxScroll = scrollHeight - clientHeight;
    if (maxScroll <= 0) return 0;
    return Math.min(Math.max(scrollPercentage * maxScroll, 0), maxScroll);
  }

  /**
   * 检查是否为旧版本的数据格式
   * @param progress 进度数据
   * @returns 是否为旧版本格式
   */
  static isLegacyProgress(progress: MangaProgress): boolean {
    return (
      progress.scrollPosition !== undefined && progress.scrollPercentage === 0
    );
  }
}
