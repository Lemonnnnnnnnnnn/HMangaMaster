<template>
  <div class="dropdown-container">
    <!-- 触发按钮插槽 -->
    <div
      ref="triggerRef"
      @click="handleTriggerClick"
      class="dropdown-trigger inline-block"
    >
      <slot name="trigger"></slot>
    </div>

    <!-- 下拉菜单内容（Teleport 到 body） -->
    <Teleport to="body">
      <Transition name="dropdown">
        <div
          v-if="modelValue"
          ref="dropdownRef"
          :style="dropdownStyle"
          class="dropdown-menu bg-neutral-800 border border-neutral-500/70 rounded-lg shadow-xl z-50"
          @click="handleMenuClick"
        >
          <slot></slot>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, nextTick, toRefs } from 'vue';

interface DropDownProps {
  modelValue: boolean;
  disabled?: boolean;
  position?: 'bottom-left' | 'bottom-right' | 'top-left' | 'top-right';
  align?: 'start' | 'end';
  offset?: number;
  closeOnClick?: boolean;
  closeOnEscape?: boolean;
  closeOnOutsideClick?: boolean;
}

interface DropDownEmits {
  'update:modelValue': [value: boolean];
  'select': [value: any];
  'open': [];
  'close': [];
}

const props = withDefaults(defineProps<DropDownProps>(), {
  disabled: false,
  position: 'bottom-right',
  align: 'end',
  offset: 8,
  closeOnClick: true,
  closeOnEscape: true,
  closeOnOutsideClick: true,
});

const emit = defineEmits<DropDownEmits>();

// 使用 toRefs 解构 props，这样可以直接使用 modelValue
const { modelValue } = toRefs(props);

const triggerRef = ref<HTMLElement>();
const dropdownRef = ref<HTMLElement>();

// 位置计算
const dropdownStyle = computed(() => {
  if (!triggerRef.value || !modelValue.value) return {};

  const triggerRect = triggerRef.value.getBoundingClientRect();
  const dropdownWidth = dropdownRef.value?.offsetWidth || 140;
  const dropdownHeight = dropdownRef.value?.offsetHeight || 0;
  const viewportWidth = window.innerWidth;
  const viewportHeight = window.innerHeight;

  let top = 0;
  let left = 0;

  // 垂直位置计算
  if (props.position?.startsWith('bottom')) {
    top = triggerRect.bottom + props.offset;
    // 检测是否超出视口底部，自动翻转到上方
    if (top + dropdownHeight > viewportHeight - 8) {
      top = triggerRect.top - dropdownHeight - props.offset;
    }
  } else {
    top = triggerRect.top - dropdownHeight - props.offset;
    // 检测是否超出视口顶部，自动翻转到下方
    if (top < 8) {
      top = triggerRect.bottom + props.offset;
    }
  }

  // 水平位置计算
  if (props.align === 'start') {
    left = triggerRect.left;
    // 检测是否超出右边缘
    if (left + dropdownWidth > viewportWidth - 8) {
      left = viewportWidth - dropdownWidth - 8;
    }
  } else {
    left = triggerRect.right - dropdownWidth;
    // 检测是否超出左边缘
    if (left < 8) {
      left = 8;
    }
  }

  return {
    position: 'fixed' as any,
    top: `${top}px`,
    left: `${left}px`,
    minWidth: `${Math.max(triggerRect.width, 140)}px`,
  };
});

// 监听 modelValue 变化
watch(modelValue, async (newValue) => {
  if (newValue) {
    await nextTick();
    // 触发 dropdownStyle 重新计算
    dropdownStyle.value;
    emit('open');
    // 添加 ESC 键监听
    document.addEventListener('keydown', handleKeydown);
  } else {
    emit('close');
    // 移除 ESC 键监听
    document.removeEventListener('keydown', handleKeydown);
  }
});

// 点击外部关闭
const handleClickOutside = (event: MouseEvent) => {
  if (!props.closeOnOutsideClick || !modelValue.value) return;

  const target = event.target as Node;
  const dropdownElement = dropdownRef.value;
  const triggerElement = triggerRef.value;

  if (
    dropdownElement &&
    !dropdownElement.contains(target) &&
    triggerElement &&
    !triggerElement.contains(target)
  ) {
    closeDropdown();
  }
};

// ESC 键关闭
const handleKeydown = (event: KeyboardEvent) => {
  if (!props.closeOnEscape) return;
  if (event.key === 'Escape' && modelValue.value) {
    closeDropdown();
    triggerRef.value?.focus();
  }
};

// 菜单项点击处理
const handleMenuClick = (event: MouseEvent) => {
  const target = event.target as HTMLElement;
  const value = target.closest('[data-value]')?.getAttribute('data-value');

  if (value && props.closeOnClick) {
    emit('select', value);
    closeDropdown();
  }
};

// 触发器点击处理
const handleTriggerClick = () => {
  if (props.disabled) return;
  emit('update:modelValue', !modelValue.value);
};

const closeDropdown = () => {
  emit('update:modelValue', false);
  emit('close');
};

// 窗口调整大小处理
const handleResize = () => {
  if (modelValue.value) {
    nextTick(() => {
      dropdownStyle.value;
    });
  }
};

onMounted(() => {
  document.addEventListener('click', handleClickOutside);
  window.addEventListener('resize', handleResize);
});

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside);
  document.removeEventListener('keydown', handleKeydown);
  window.removeEventListener('resize', handleResize);
});
</script>

<style scoped>
.dropdown-enter-active,
.dropdown-leave-active {
  transition: all 0.2s ease;
}

.dropdown-enter-from,
.dropdown-leave-to {
  opacity: 0;
  transform: scaleY(0.95) translateY(-8px);
}

.dropdown-enter-to,
.dropdown-leave-from {
  opacity: 1;
  transform: scaleY(1) translateY(0);
}

/* 防止 trigger 影响布局 */
.dropdown-trigger {
  display: inline-block;
}

/* 确保菜单项有基本样式 */
.dropdown-menu button {
  outline: none;
}
</style>
