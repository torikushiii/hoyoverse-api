import { CronJob } from 'cron';
import { EventScraper } from './services/scraper';
import { DatabaseService } from './services/database';

async function scrapeAndStore(): Promise<void> {
    console.log('Starting event scraping...');
    try {
        const db = await DatabaseService.getInstance();
        const { events, error } = await EventScraper.scrapeEvents();

        if (error) {
            console.error('Error occurred while scraping:', error.message);
            return;
        }

        if (events.length === 0) {
            console.log('No events found');
            return;
        }

        const newEvents = await db.findNewEvents(events);
        if (newEvents.length === 0) {
            console.log('No new events found');
            return;
        }

        await db.insertEvents(newEvents);
        console.log(`Successfully stored ${newEvents.length} new events`);

    } catch (error) {
        console.error('Error in scrape and store process:', error);
    }
}

const job = new CronJob(
    '*/10 * * * *',
    scrapeAndStore,
    null,
    false,
    'UTC'
);

let isShuttingDown = false;
if (!isShuttingDown) {
    job.start();
    console.log('Cron job started');

    scrapeAndStore().catch(error => {
        console.error('Initial scrape failed:', error);
    });
}

const shutdown = () => {
    if (isShuttingDown) {
        return;
    }

    isShuttingDown = true;
    console.log('Gracefully shutting down...');
    job.stop();
};

process.on('SIGTERM', shutdown);
process.on('SIGINT', shutdown);
process.on('message', (msg) => {
    if (msg === 'shutdown') {
        shutdown();
    }
});
