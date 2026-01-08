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

    public async upsertEvents(events: Event[]): Promise<{ inserted: number; updated: number }> {
        if (events.length === 0) {
            return { inserted: 0, updated: 0 };
        }

        let inserted = 0;
        let updated = 0;

        for (const event of events) {
            try {
                const result = await this.eventsCollection.updateOne(
                    { name: event.name, game: event.game },
                    {
                        $set: {
                            imageUrl: event.imageUrl,
                            ...(event.startTime !== undefined && { startTime: event.startTime }),
                            ...(event.endTime !== undefined && { endTime: event.endTime }),
                        },
                        $setOnInsert: {
                            name: event.name,
                            game: event.game,
                        }
                    },
                    { upsert: true }
                );

                if (result.upsertedCount > 0) {
                    inserted++;
                } else if (result.modifiedCount > 0) {
                    updated++;
                }
            } catch (error) {
                console.error(`Error upserting event "${event.name}":`, error);
            }
        }

        return { inserted, updated };
    }
}
