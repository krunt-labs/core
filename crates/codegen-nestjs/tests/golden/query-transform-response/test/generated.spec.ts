import assert from 'node:assert/strict';
import { test } from 'node:test';
import { AppController } from '../src/app.controller';
import { AppModule } from '../src/app.module';

test('generated NestJS application exports its root module', () => {
  assert.equal(typeof AppModule, 'function');
  assert.equal(typeof AppController, 'function');
});
