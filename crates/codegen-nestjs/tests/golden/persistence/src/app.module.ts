import { Module } from '@nestjs/common';
import { AppController } from './app.controller';
import { KruntRuntimeService } from './krunt-runtime';
import { UserRepositoryRepository } from './repositories/UserRepositoryRepository.repository';

@Module({
  controllers: [AppController],
  providers: [KruntRuntimeService, UserRepositoryRepository],
})
export class AppModule {}
