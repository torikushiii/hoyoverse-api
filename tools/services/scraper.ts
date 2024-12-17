import { got } from 'got';
import * as cheerio from 'cheerio';
import type { Event, ScraperResult, Game } from '../types/event';

export class EventScraper {
    private static readonly SOURCES = {
        GENSHIN: "https://genshin-impact.fandom.com/wiki/Event",
        STAR_RAIL: "https://honkai-star-rail.fandom.com/wiki/Events"
    };

    private static readonly IGNORED_EVENT_TYPES = ["Test Run", "In-Person", "Web"];

    private static cleanImageUrl(url: string | undefined): string | null {
        if (!url) {
            return null;
        }
        if (url.startsWith("data:image/gif;base64")) {
            return null;
        }
        if (url.includes("/scale-to-width-down/")) {
            return url.replace(/\/scale-to-width-down\/\d+/, '/scale-to-width-down/1000');
        }

        return url.replace(/(.+?\.(?:png|jpg|jpeg|gif))(\/revision.+)?/i, "$1");
    }

    private static cleanEventName(name: string | undefined): string | null {
        if (!name) {
            return null;
        }

        return name.trim().replace(/\s+\d{4}-\d{2}-\d{2}$/, "");
    }

    private static shouldIgnoreEvent(eventName: string | null, type: string): boolean {
        if (!eventName) {
            return true;
        }

        return this.IGNORED_EVENT_TYPES.some(ignoredType =>
            eventName.includes(ignoredType) || type.includes(ignoredType)
        );
    }

    // @ts-ignore
    private static processEventTable($: cheerio.CheerioAPI, table: cheerio.Cheerio<cheerio.Element>, game: Game): Event[] {
        const events: Event[] = [];

        table.find("tbody tr").each((_, element) => {
            const $row = $(element);

            const eventName = this.cleanEventName($row.find("td:first-child a:last-child").text());
            const imageUrl = this.cleanImageUrl(
                $row.find("td:first-child img").attr("data-src") ||
                $row.find("td:first-child img").attr("src")
            );
            const type = $row.find("td:nth-child(3)").text().trim();

            if (this.shouldIgnoreEvent(eventName, type)) {
                return;
            }

            if (!eventName || !imageUrl) {
                return;
            }

            events.push({ name: eventName, imageUrl, game });
        });

        return events;
    }

    private static async scrapeSource(url: string, game: Game): Promise<Event[]> {
        const response = await got({
            url,
            responseType: 'text'
        });

        const $ = cheerio.load(response.body);
        let events: Event[] = [];

        const tables = [
            $("#mw-content-text > div > table.wikitable.sortable").first(),
            $("#Upcoming").parent().nextAll("table.wikitable.sortable").first()
        ];

        for (const table of tables) {
            if (table.length > 0) {
                events = [...events, ...this.processEventTable($, table, game)];
            }
        }

        return events;
    }

    public static async scrapeEvents(): Promise<ScraperResult> {
        try {
            const allEvents: Event[] = [];

            const genshinEvents = await this.scrapeSource(this.SOURCES.GENSHIN, 'genshin');
            allEvents.push(...genshinEvents);

            const starRailEvents = await this.scrapeSource(this.SOURCES.STAR_RAIL, 'starrail');
            allEvents.push(...starRailEvents);

            return { events: allEvents };
        } catch (error) {
            console.error("Error occurred while scraping:", error);
            return {
                events: [],
                error: error instanceof Error ? error : new Error('Unknown error occurred')
            };
        }
    }
}