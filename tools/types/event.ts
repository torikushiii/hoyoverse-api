export type Game = 'genshin' | 'starrail';

export interface Event {
    name: string;
    imageUrl: string;
    game: Game;
    startTime?: number;
    endTime?: number;
}

export interface ScraperResult {
    events: Event[];
    error?: Error;
}
