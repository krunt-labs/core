// krunt:source=node:4
export interface User {
  id: number;
  tags: Array<string>;
}

export function validateUser(value: unknown): User {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    throw new Error('User must be an object.');
  }
  const record = value as Record<string, unknown>;
  if (!Object.prototype.hasOwnProperty.call(record, 'id') || typeof record.id !== 'number' || !Number.isInteger(record.id)) {
    throw new Error('User.id must be a integer.');
  }
  if (!Object.prototype.hasOwnProperty.call(record, 'tags') || !Array.isArray(record.tags) || !record.tags.every((item) => !(typeof item !== 'string'))) {
    throw new Error('User.tags must be a array.');
  }
  return {
    id: record.id as number,
    tags: record.tags as Array<string>,
  };
}
