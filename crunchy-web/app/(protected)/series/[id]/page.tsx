'use client';

import {useState, useEffect} from 'react';
import {useParams} from 'next/navigation';
import {CircleCheckBigIcon} from 'lucide-react';
import {PagePanel} from '@/components/layout/page';
import {CRImage, CRImageBackdrop} from '@/components/ui/cr-image';
import {Button} from '@/components/ui/button';
import {SeasonSelector} from '@/components/series/season-selector';
import {EpisodeListItem, EpisodeListSkeleton} from '@/components/series/episode-list-item';
import {DownloadSeriesButton} from '@/components/downloads/download-series-button';
import {BookmarkButton} from '@/components/bookmarks/bookmark-button';
import {TrackButton} from '@/components/tracking/track-button';
import {useSeries, useSeasons, useEpisodes} from '@/hooks/use-series';
import {useDownloadedEpisodes, useMarkManual} from '@/hooks/use-downloads';

export default function SeriesDetailPage() {
    const {id} = useParams<{ id: string }>();
    const {data: series, isLoading: seriesLoading, error: seriesError} = useSeries(id);
    const {data: seasons, isLoading: seasonsLoading} = useSeasons(id);
    const [selectedSeasonId, setSelectedSeasonId] = useState<string | null>(null);
    const {data: episodes, isLoading: episodesLoading} = useEpisodes(selectedSeasonId);
    const {completedIds, manualIds, refetch: refetchDownloaded} = useDownloadedEpisodes();
    const {markBulk, isLoading: marking} = useMarkManual();
    const [screenWidth, setScreenWidth] = useState<number | null>(null);

    const handleMarkSeason = async () => {
        if (!episodes?.length) return;
        const items = episodes
            .filter((ep) => !completedIds.has(ep.id) && !manualIds.has(ep.id))
            .map((ep) => ({
                episode_id: ep.id,
                series_title: ep.series_title,
                episode_title: ep.title,
                season_number: ep.season_number,
                episode_number: ep.episode_number ?? undefined,
                thumbnail_url:
                    ep.images.thumbnail?.[0]?.[0]?.source ?? null,
            }));
        if (items.length === 0) return;
        const {error} = await markBulk(items);
        if (!error) refetchDownloaded();
    };

    useEffect(() => {
        setScreenWidth(window.screen.width);
    }, []);

    // Auto-select first season when loaded
    useEffect(() => {
        if (seasons?.length && !selectedSeasonId) {
            setSelectedSeasonId(seasons[0].id);
        }
    }, [seasons, selectedSeasonId]);

    if (seriesLoading || !screenWidth) {
        return (
            <PagePanel>
                <div className="animate-pulse space-y-4">
                    <div className="h-48 w-full bg-muted rounded-xl"/>
                    <div className="h-8 w-64 bg-muted rounded"/>
                    <div className="h-4 w-full bg-muted rounded"/>
                </div>
            </PagePanel>
        );
    }

    if (seriesError || !series) {
        return (
            <PagePanel>
                <div className="flex flex-col items-center py-16 text-muted-foreground">
                    <p className="text-sm">{seriesError ?? 'Series not found'}</p>
                </div>
            </PagePanel>
        );
    }

    return (
        <PagePanel>
            {/* Hero */}
            <div className="relative rounded-xl overflow-hidden mb-6">
                <div
                    className="w-full grid grid-cols-12 aspect-673/267 max-h-275 6xl:max-h-325 relative overflow-hidden transition-all duration-300 ease-out">
                    <div className="absolute w-[120%] 5xl:w-full h-full z-0 transition-all duration-300 ease-out">
                        {/*<img*/}
                        {/*    className="block w-full h-full object-cover transition-all duration-300 ease-out"*/}
                        {/*    loading="eager"*/}
                        {/*    src={bgsrc}*/}
                        {/*    alt=""*/}
                        {/*    data-t="original-image"*/}
                        {/*    fetchPriority="high"*/}
                        {/*    sizes="100vw"*/}
                        {/*    srcSet={bgset}*/}
                        {/*/>*/}
                        <CRImageBackdrop
                            series={series}
                            alt={series.title}
                            preferredWidth={screenWidth}
                            className="w-full h-full object-cover"
                        />
                    </div>
                    <div className="gradient-overlay"></div>
                    {/*<div className="z-40 col-span-3 transition-all duration-300 ease-out aspect-15/8 mt-auto ml-8 mb-8">*/}
                    {/*    <img*/}
                    {/*        src={logosrc}*/}
                    {/*        srcSet={logoset}*/}
                    {/*        alt=""*/}
                    {/*        sizes="(max-width: 960px) 320px, (max-width: 1260px) 480px, 600px"*/}
                    {/*        className="object-bottom-left z-10 w-full h-full max-h-full object-contain transition-all duration-300 ease-out pointer-events-none select-none mt-auto"*/}
                    {/*    />*/}
                    {/*</div>*/}
                </div>
                <div className="absolute bottom-0 left-0 p-6">
                    <h1 className="text-2xl md:text-3xl font-bold font-display drop-shadow-lg">
                        {series.title}
                    </h1>
                    <div className="flex items-center gap-3 mt-2 text-sm text-foreground/80">
                        <span>{series.season_count} season{series.season_count !== 1 ? 's' : ''}</span>
                        <span className="text-foreground/40">|</span>
                        <span>{series.episode_count} episode{series.episode_count !== 1 ? 's' : ''}</span>
                        {series.is_simulcast && (
                            <>
                                <span className="text-foreground/40">|</span>
                                <span className="text-primary">Simulcast</span>
                            </>
                        )}
                    </div>
                </div>
            </div>

            {/* Description */}
            {series.description && (
                <p className="text-sm text-muted-foreground leading-relaxed mb-6">
                    {series.description}
                </p>
            )}

            {/* Seasons */}
            <div className="mb-4">
                {seasonsLoading ? (
                    <div className="flex gap-2">
                        {Array.from({length: 3}).map((_, i) => (
                            <div key={i} className="h-8 w-24 bg-muted rounded-lg animate-pulse"/>
                        ))}
                    </div>
                ) : seasons?.length ? (
                    <div className="flex items-center gap-3 flex-wrap">
                        <SeasonSelector
                            seasons={seasons}
                            selectedId={selectedSeasonId}
                            onSelect={setSelectedSeasonId}
                        />
                        {selectedSeasonId && (
                            <DownloadSeriesButton
                                seriesId={id}
                                seasonId={selectedSeasonId}
                                episodeCount={
                                    seasons.find((s) => s.id === selectedSeasonId)?.number_of_episodes ?? 0
                                }
                            />
                        )}
                        {selectedSeasonId && episodes?.length ? (
                            <Button
                                variant="outline"
                                size="sm"
                                onClick={handleMarkSeason}
                                disabled={marking}
                                title="Mark every episode in this season as already downloaded"
                            >
                                <CircleCheckBigIcon/>
                                Mark season as downloaded
                            </Button>
                        ) : null}
                        <BookmarkButton seriesId={id} variant="outline" size="sm"/>
                        <TrackButton seriesId={id} variant="outline" size="sm"/>
                    </div>
                ) : null}
            </div>

            {/* Episodes */}
            <div className="space-y-2">
                {episodesLoading ? (
                    Array.from({length: 5}).map((_, i) => <EpisodeListSkeleton key={i}/>)
                ) : episodes?.length ? (
                    episodes.map((ep) => (
                        <EpisodeListItem
                            key={ep.id}
                            episode={ep}
                            isDownloaded={completedIds.has(ep.id)}
                            isMarked={manualIds.has(ep.id)}
                            onChanged={refetchDownloaded}
                        />
                    ))
                ) : selectedSeasonId ? (
                    <p className="text-sm text-muted-foreground py-8 text-center">
                        No episodes found for this season.
                    </p>
                ) : null}
            </div>
        </PagePanel>
    );
}
