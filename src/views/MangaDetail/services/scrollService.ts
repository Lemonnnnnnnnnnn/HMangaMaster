import type { Ref } from "vue";
import { ProgressService } from ".";
import type { useMangaStore } from "../stores";
import type { MangaService } from "./mangaService";

export class ScrollService {
  private scrollContainer: Ref<HTMLElement | null, HTMLElement | null>;
  private mangaStore: ReturnType<typeof useMangaStore>;
  private saveTimeout: number | null = null;
  private smoothScroller: SmoothScroller;

  constructor(
    scrollContainer: Ref<HTMLElement | null, HTMLElement | null>,
    mangaStore: ReturnType<typeof useMangaStore>,
  ) {
    this.scrollContainer = scrollContainer;
    this.mangaStore = mangaStore;
    this.smoothScroller = new SmoothScroller(this.scrollContainer);
  }

  registerEvent = () => {
    window.addEventListener("keydown", this.handleKeyDown);
    window.addEventListener("keyup", this.handleKeyUp);
    return () => {
      window.removeEventListener("keydown", this.handleKeyDown);
      window.removeEventListener("keyup", this.handleKeyUp);
    };
  };

  handleKeyDown = (event: KeyboardEvent) => {
    if (event.key === "j") {
      this.smoothScroller.scrollDown();
    } else if (event.key === "k") {
      this.smoothScroller.scrollUp();
    }
  };

  handleKeyUp = (event: KeyboardEvent) => {
    if (event.key === "j" || event.key === "k") {
      this.smoothScroller.stopScroll();
    }
  };

  restoreScrollPosition() {
    const progress = ProgressService.getProgress(this.mangaStore.mangaPath);
    if (progress && this.scrollContainer?.value) {
      const container = this.scrollContainer.value;
      let scrollPosition = 0;
      let logMessage = "";

      // 检查是否为旧版本数据格式
      if (
        ProgressService.isLegacyProgress(progress) &&
        progress.scrollPosition !== undefined
      ) {
        // 旧版本数据：直接使用保存的像素位置，但限制在有效范围内
        const maxScroll = container.scrollHeight - container.clientHeight;
        scrollPosition = Math.min(progress.scrollPosition, maxScroll);
        logMessage = `已恢复到上次阅读位置：${scrollPosition}px (旧版本数据)`;

        // 异步迁移为新格式
        setTimeout(() => {
          const scrollPercentage = ProgressService.calculateScrollPercentage(
            scrollPosition,
            container.scrollHeight,
            container.clientHeight,
          );
          ProgressService.saveProgress(
            this.mangaStore.mangaPath,
            scrollPercentage,
            this.mangaStore.selectedImages.length,
          );
        }, 100);
      } else if (progress.scrollPercentage > 0) {
        // 新版本数据：根据百分比计算实际滚动位置
        scrollPosition = ProgressService.calculateScrollPosition(
          progress.scrollPercentage,
          container.scrollHeight,
          container.clientHeight,
        );
        logMessage = `已恢复到上次阅读位置：${scrollPosition}px (百分比: ${(progress.scrollPercentage * 100).toFixed(1)}%)`;
      }

      if (scrollPosition > 0) {
        container.scrollTo({
          top: scrollPosition,
          // behavior: "smooth" // 注释掉平滑滚动，确保立即生效
        });
        console.log(logMessage);
      }
    }
  }

  debounceSaveProgress() {
    if (this.saveTimeout) {
      clearTimeout(this.saveTimeout);
    }

    this.saveTimeout = setTimeout(() => {
      if (this.scrollContainer?.value && this.mangaStore.mangaPath) {
        const container = this.scrollContainer.value;
        const scrollPosition = container.scrollTop || 0;

        // 计算滚动百分比
        const scrollPercentage = ProgressService.calculateScrollPercentage(
          scrollPosition,
          container.scrollHeight,
          container.clientHeight,
        );

        ProgressService.saveProgress(
          this.mangaStore.mangaPath,
          scrollPercentage,
          this.mangaStore.selectedImages.length,
        );
      }
    }, 100);
  }
}

export class SmoothScroller {
  private container: Ref<HTMLElement | null, HTMLElement | null>;
  private targetScrollPos: number;
  private isScrolling: boolean;
  private scrollDirection: number; // 0: 无滚动, 1: 向下, -1: 向上
  private scrollAmount: number; // 每次滚动量
  private scrollDuration: number; // 每次滚动周期(ms)
  private frameDuration: number;

  constructor(
    container: Ref<HTMLElement | null, HTMLElement | null>,
    scrollAmount = 64,
    scrollDuration = 128,
  ) {
    this.container = container;
    this.targetScrollPos = this.container.value?.scrollTop || 0;
    this.isScrolling = false;
    this.scrollDirection = 0;
    this.scrollAmount = scrollAmount;
    this.scrollDuration = scrollDuration;
    this.frameDuration = 16;
  }

  // 缓动函数 (线性)
  easeLinear(t: number) {
    return t;
  }

  animateScroll = () => {
    if (this.scrollDirection === 0) {
      this.isScrolling = false;
      return;
    }
    const currentPos = this.container.value?.scrollTop || 0;
    const distance = this.targetScrollPos - currentPos;

    if (Math.abs(distance) < 0.1) {
      // 滚动完成
      this.isScrolling = false;
      if (this.container.value) {
        this.container.value.scrollTop = this.targetScrollPos;
      }

      // 如果有待处理的滚动，继续
      if (this.scrollDirection !== 0) {
        this.startScroll(this.scrollDirection);
      }
      return;
    }

    // 计算这一帧应该滚动的距离
    const frameCount = this.scrollDuration / this.frameDuration;
    const scrollThisFrame =
      (this.scrollAmount / frameCount) * this.scrollDirection;

    if (this.container.value) {
      this.container.value.scrollTop = currentPos + scrollThisFrame;
    }

    // 继续下一帧
    requestAnimationFrame(this.animateScroll);
  };

  /**
   * 开始滚动
   * @param {number} direction - 1: 向下, -1: 向上
   */
  startScroll = (direction: number) => {
    this.scrollDirection = direction;
    this.targetScrollPos =
      (this.container.value?.scrollTop || 0) + this.scrollAmount * direction ||
      0;

    if (!this.isScrolling) {
      this.isScrolling = true;
      requestAnimationFrame(this.animateScroll);
    }
  };

  stopScroll = () => {
    this.scrollDirection = 0;
  };

  scrollDown = () => this.startScroll(1);
  scrollUp = () => this.startScroll(-1);
}
