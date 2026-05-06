'use client';

import {useMemo, useState} from 'react';
import {ImageIcon} from 'lucide-react';
import {cn} from '@/lib/utils';
import {getCrunchyrollImageUrl, proxyImageUrl} from '@/lib/image-helpers';
import {CRImages, CRSeries} from '@/types/crunchyroll';

type CRImageProps = {
    images: CRImages;
    type: 'tall' | 'wide' | 'thumbnail';
    preferredWidth?: number;
    alt: string;
    className?: string;
};

export function CRImage({images, type, preferredWidth, alt, className}: CRImageProps) {
    const [hasError, setHasError] = useState(false);
    const src = getCrunchyrollImageUrl(images, type, preferredWidth);

    if (!src || hasError) {
        return (
            <div
                className={cn(
                    'flex items-center justify-center bg-muted text-muted-foreground',
                    className
                )}
            >
                <ImageIcon className="size-8 opacity-40"/>
            </div>
        );
    }

    return (
        // eslint-disable-next-line @next/next/no-img-element
        <img
            src={proxyImageUrl(src)}
            alt={alt}
            className={className}
            onError={() => setHasError(true)}
            loading="lazy"
        />
    );
}

type CRImageBackdropProps = {
    series: CRSeries;
    alt: string;
    className?: string;
    preferredWidth?: number;
}

export function CRImageBackdrop({series: {id}, alt, className, preferredWidth = 1920}: CRImageBackdropProps) {
    const [hasError, setHasError] = useState(false);
    const [loaded, setLoaded] = useState(false);

    const basePath = `https://imgsrv.crunchyroll.com/cdn-cgi/image/fit=cover,format=auto`;
    const src = `${basePath},quality=85,width=${preferredWidth}/keyart/${id}-backdrop_wide`;
    const placeholderSrc = `${basePath},quality=85,width=${preferredWidth},blur=100/keyart/${id}-backdrop_wide`;

    if (hasError) {
        return (
            <div
                className={cn(
                    'flex items-center justify-center bg-muted text-muted-foreground',
                    className
                )}
            >
                <ImageIcon className="size-8 opacity-40"/>
            </div>
        );
    }

    return (
        <div className={cn('relative overflow-hidden', className)}>
            {/* Blurred low-res placeholder */}
            <img
                src={proxyImageUrl(placeholderSrc)}
                alt=""
                aria-hidden
                className={cn(
                    'absolute inset-0 h-full w-full object-cover scale-105 blur-xl transition-opacity duration-500',
                    loaded ? 'opacity-0' : 'opacity-100'
                )}
            />
            {/* Full-res image */}
            <img
                src={proxyImageUrl(src)}
                alt={alt}
                className={cn(
                    'absolute inset-0 h-full w-full object-cover transition-opacity duration-500',
                    loaded ? 'opacity-100' : 'opacity-0'
                )}
                onLoad={() => setLoaded(true)}
                onError={() => setHasError(true)}
                loading="lazy"
            />
        </div>
    )
}
