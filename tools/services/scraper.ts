import { got } from 'got';
import * as cheerio from 'cheerio';
import type { Event, ScraperResult, Game } from '../types/event';

export class EventScraper {
    private static readonly SOURCES = {
        GENSHIN: "https://genshin-impact.fandom.com/wiki/Event",
        STAR_RAIL: "https://honkai-star-rail.fandom.com/wiki/Events"
    };

    private static readonly IGNORED_EVENT_TYPES = ["Test Run", "In-Person", "Web"];
    private static readonly WSRV_BASE = "https://wsrv.nl/?url=";

    private static cleanImageUrl(url: string | undefined): string | null {
        if (!url) {
            return null;
        }
        if (url.startsWith("data:image/gif;base64")) {
            return null;
        }

        let cleanedUrl: string;
        if (url.includes("/scale-to-width-down/")) {
            cleanedUrl = url.replace(/\/scale-to-width-down\/\d+/, '/scale-to-width-down/1000');
        } else {
            cleanedUrl = url.replace(/(.+?\.(?:png|jpg|jpeg|gif))(\/revision.+)?/i, "$1");
        }

        return `${this.WSRV_BASE}${encodeURIComponent(cleanedUrl)}&output=webp&q=85`;
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

    private static parseTimestamp(dateStr: string): number {
        try {
            const timestamp = new Date(dateStr.trim()).getTime();
            return !isNaN(timestamp) ? timestamp : 0;
        } catch {
            return 0;
        }
    }

    // @ts-ignore
    private static processEventTable($: cheerio.CheerioAPI, table: cheerio.Cheerio<cheerio.Element>, game: Game, isUpcoming: boolean): Event[] {
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

            const event: Event = { name: eventName, imageUrl, game };

            if (isUpcoming && game === 'genshin') {
                const durationValue = $row.find("td:nth-child(2)").attr("data-sort-value");
                if (durationValue) {
                    try {
                        const [startTime, endTime] = durationValue.split(/(?=\d{4}-\d{2}-\d{2})/);
                        if (startTime) {
                            event.startTime = this.parseTimestamp(startTime);
                        }
                        if (endTime) {
                            event.endTime = this.parseTimestamp(endTime);
                        }
                    } catch {
                        console.warn(`Failed to parse timestamps for Genshin event: ${eventName}`);
                    }
                }
            }

            events.push(event);
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

        const currentTable = $("#mw-content-text > div > table.wikitable.sortable").first();
        const upcomingTable = $("#Upcoming").parent().nextAll("table.wikitable.sortable").first();

        if (currentTable.length > 0) {
            events = [...events, ...this.processEventTable($, currentTable, game, false)];
        }

        if (upcomingTable.length > 0) {
            events = [...events, ...this.processEventTable($, upcomingTable, game, true)];
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