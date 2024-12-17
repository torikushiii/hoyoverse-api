import { MongoClient, Collection } from 'mongodb';
import type { Event } from '../types/event';
import * as yaml from 'yaml';
import * as fs from 'fs';
import * as path from 'path';

export class DatabaseService {
    private static instance: DatabaseService;
    private client: MongoClient;
    private eventsCollection!: Collection<Event>;

    private constructor(mongoUrl: string, private dbName: string) {
        this.client = new MongoClient(mongoUrl);
    }

    public static async getInstance(): Promise<DatabaseService> {
        if (!this.instance) {
            const config = yaml.parse(
                fs.readFileSync(path.resolve(__dirname, '../../config/prod.yaml'), 'utf8')
            );

            this.instance = new DatabaseService(
                config.mongodb.url,
                config.mongodb.database
            );
            await this.instance.initialize();
        }
        return this.instance;
    }

    private async initialize(): Promise<void> {
        await this.client.connect();
        const db = this.client.db(this.dbName);
        this.eventsCollection = db.collection<Event>('events');

        await this.eventsCollection.createIndex(
            { name: 1, game: 1 },
            { unique: false }
        );
    }

    public async findNewEvents(events: Event[]): Promise<Event[]> {
        const newEvents: Event[] = [];

        for (const event of events) {
            const exists = await this.eventsCollection.findOne({
                name: event.name,
                game: event.game
            });

            if (!exists) {
                newEvents.push(event);
            }
        }

        return newEvents;
    }

    public async insertEvents(events: Event[]): Promise<void> {
        if (events.length === 0) {
            return;
        }

        try {
            await this.eventsCollection.insertMany(events, { ordered: false });
        } catch (error) {
            console.error('Error inserting events:', error);
            throw error;
        }
    }
}
