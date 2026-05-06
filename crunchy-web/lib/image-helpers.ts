import type { CRImages, CRImage } from '@/types/crunchyroll';

const ALLOWED_HOSTS = [
  'static.crunchyroll.com',
  'img1.ak.crunchyroll.com',
  'www.crunchyroll.com',
  'a-static.crunchyroll.com',
];

export function isAllowedImageUrl(url: string): boolean {
  try {
    const parsed = new URL(url);
    return ALLOWED_HOSTS.some(
      (host) => parsed.hostname === host || parsed.hostname.endsWith('.crunchyroll.com')
    );
  } catch {
    return false;
  }
}

export function getBestImage(
  images: CRImages,
  type: 'tall' | 'wide' | 'thumbnail',
  preferredWidth?: number
): CRImage | null {
  if (!images) return null;
  const key = type === 'tall' ? 'poster_tall' : type === 'wide' ? 'poster_wide' : 'thumbnail';
  const imageArrays = images[key];
  if (!imageArrays?.length) return null;

  // Flatten — each entry is an array of size variants
  const variants = imageArrays[0];
  if (!variants?.length) return null;

  if (!preferredWidth) {
    // Return largest
    return variants.reduce((best, img) => (img.width > best.width ? img : best), variants[0]);
  }

  // Return closest to preferred width
  return variants.reduce((best, img) => {
    const bestDiff = Math.abs(best.width - preferredWidth);
    const imgDiff = Math.abs(img.width - preferredWidth);
    return imgDiff < bestDiff ? img : best;
  }, variants[0]);
}

export function getCrunchyrollImageUrl(
  images: CRImages,
  type: 'tall' | 'wide' | 'thumbnail',
  preferredWidth?: number
): string | null {
  const image = getBestImage(images, type, preferredWidth);
  return image?.source ?? null;
}

export function proxyImageUrl(originalUrl: string): string {
  return `/api/image?url=${encodeURIComponent(originalUrl)}`;
}
