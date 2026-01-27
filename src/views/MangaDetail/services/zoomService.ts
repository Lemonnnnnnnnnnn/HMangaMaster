import { ref, type Ref, readonly } from "vue";

export interface ZoomConfig {
  minZoom?: number;      // 默认 0.5 (50%)
  maxZoom?: number;      // 默认 2.0 (200%)
  zoomStep?: number;     // 默认 0.1 (10%)
  initialZoom?: number;  // 默认 1.0 (100%)
}

export class ZoomService {
  private zoomLevel: Ref<number>;
  private minZoom: number;
  private maxZoom: number;
  private zoomStep: number;

  constructor(config: ZoomConfig = {}) {
    this.minZoom = config.minZoom ?? 0.5;
    this.maxZoom = config.maxZoom ?? 2.0;
    this.zoomStep = config.zoomStep ?? 0.1;

    this.zoomLevel = ref(config.initialZoom ?? 1.0);
  }

  /**
   * 放大图片
   */
  zoomIn(): void {
    if (this.canZoomIn()) {
      this.zoomLevel.value = Math.min(
        this.zoomLevel.value + this.zoomStep,
        this.maxZoom
      );
    }
  }

  /**
   * 缩小图片
   */
  zoomOut(): void {
    if (this.canZoomOut()) {
      this.zoomLevel.value = Math.max(
        this.zoomLevel.value - this.zoomStep,
        this.minZoom
      );
    }
  }

  /**
   * 重置到默认缩放级别
   */
  reset(): void {
    this.zoomLevel.value = 1.0;
  }

  /**
   * 获取当前缩放级别（只读响应式引用）
   */
  getZoomLevel(): Readonly<Ref<number>> {
    return readonly(this.zoomLevel);
  }

  /**
   * 获取当前缩放百分比（0-100）
   */
  getZoomPercentage(): number {
    return Math.round(this.zoomLevel.value * 100);
  }

  /**
   * 检查是否可以继续放大
   */
  canZoomIn(): boolean {
    return this.zoomLevel.value < this.maxZoom;
  }

  /**
   * 检查是否可以继续缩小
   */
  canZoomOut(): boolean {
    return this.zoomLevel.value > this.minZoom;
  }
}
