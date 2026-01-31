import { clsx, type ClassValue } from 'clsx'
import { twMerge } from 'tailwind-merge'

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

// Admin perPage localStorage utilities
const ADMIN_PER_PAGE_KEY = 'admin_per_page'
const DEFAULT_PER_PAGE = 20
export const PER_PAGE_OPTIONS = [20, 40, 60, 80, 100] as const

export function getAdminPerPage(): number {
  try {
    const stored = localStorage.getItem(ADMIN_PER_PAGE_KEY)
    if (stored) {
      const value = parseInt(stored, 10)
      if (PER_PAGE_OPTIONS.includes(value as typeof PER_PAGE_OPTIONS[number])) {
        return value
      }
    }
  } catch {
    // localStorage not available
  }
  return DEFAULT_PER_PAGE
}

export function setAdminPerPage(value: number): void {
  try {
    localStorage.setItem(ADMIN_PER_PAGE_KEY, String(value))
  } catch {
    // localStorage not available
  }
}

