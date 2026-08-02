export interface NotificationEvent {
  deviceId: string;
  resourceId: string;
  previousState: string;
  currentState: string;
  message: string;
}
