export type Game = 'genshin' | 'starrail';

export interface Event {
    name: string;
    imageUrl: string;
    game: Game;
}

export interface ScraperResult {
    events: Event[];
    error?: Error;
}
