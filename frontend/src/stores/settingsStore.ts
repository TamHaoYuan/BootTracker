import { create } from 'zustand';
import type { AppSettings } from '../api/types';

interface SettingsState {
  settings: AppSettings | null;
  loading: boolean;
  error: string | null;
  setSettings: (s: AppSettings) => void;
  patch: (updates: Partial<AppSettings>) => void;
  setLoading: (b: boolean) => void;
  setError: (e: string | null) => void;
}

export const useSettingsStore = create<SettingsState>((set) => ({
  settings: null,
  loading: false,
  error: null,
  setSettings: (s) => set({ settings: s, error: null }),
  patch: (updates) =>
    set((state) =>
      state.settings ? { settings: { ...state.settings, ...updates } } : {},
    ),
  setLoading: (loading) => set({ loading }),
  setError: (error) => set({ error }),
}));
