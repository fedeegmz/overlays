import { onScopeDispose, ref } from "vue";

/**
 * Two-step confirmation: the first click arms the key, a second click on the
 * same key within `timeoutMs` runs the action.
 */
export function useConfirmClick<T>(action: (key: T) => void, timeoutMs = 2000) {
  const pending = ref<T | null>(null);
  let timer: ReturnType<typeof setTimeout> | null = null;

  function clearTimer() {
    if (timer) clearTimeout(timer);
    timer = null;
  }

  function confirmClick(key: T) {
    if (pending.value !== key) {
      pending.value = key;
      clearTimer();
      timer = setTimeout(() => {
        pending.value = null;
      }, timeoutMs);
      return;
    }
    clearTimer();
    pending.value = null;
    action(key);
  }

  onScopeDispose(clearTimer);

  return { pending, confirmClick };
}
