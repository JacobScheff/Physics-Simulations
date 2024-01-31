import pygame
import sys
import time
import math

screenSize = (1200, 600)
ballSize = 20
horizontalAmount = 5
verticalAmount = 3
fps = 1000

class Ball:
    def __init__(self, x, y, v, a, radius):
        self.x = x
        self.y = y
        self.v = v
        self.a = a # Angle in degrees
        self.vx = self.v * math.cos(math.radians(self.a))
        self.vy = self.v * math.sin(math.radians(self.a))
        self.radius = radius
        # Mass is the area of the ball
        self.mass = self.radius ** 2 * 3.14

    def draw(self, screen):
        pygame.draw.circle(screen, (255, 255, 255), (self.x, self.y), self.radius)

    def move(self, dt):
        self.x += self.vx * dt
        self.y += self.vy * dt

        if self.x < self.radius:
            self.vx = abs(self.vx)
        elif self.x > screenSize[0] - self.radius:
            self.vx = -abs(self.vx)
        if self.y < self.radius:
            self.vy = abs(self.vy)
        elif self.y > screenSize[1] - self.radius:
            self.vy = -abs(self.vy)

        self.a = math.degrees(math.atan2(self.vy, self.vx))

    def checkCollision(self, other):
        if (self.x - other.x) ** 2 + (self.y - other.y) ** 2 <= (self.radius + other.radius) ** 2:
            return True
        else:
            return False
    
    def collide(self, other):
        if self.checkCollision(other):
            distance = ((self.x - other.x) ** 2 + (self.y - other.y) ** 2) ** 0.5
            originalVx = self.vx
            originalVy = self.vy
            contactAngle = math.degrees(math.atan2(other.y - self.y, other.x - self.x))

            # Apply direct collision
            self.vx = (self.v * math.cos(math.radians(self.a - contactAngle)) * (self.mass - other.mass) + 2 * other.mass * other.v * math.cos(math.radians(other.a - contactAngle))) / (self.mass + other.mass) * math.cos(math.radians(contactAngle)) + self.v * math.sin(math.radians(self.a - contactAngle)) * math.cos(math.radians(contactAngle + 90))
            self.vy = (self.v * math.cos(math.radians(self.a - contactAngle)) * (self.mass - other.mass) + 2 * other.mass * other.v * math.cos(math.radians(other.a - contactAngle))) / (self.mass + other.mass) * math.sin(math.radians(contactAngle)) + self.v * math.sin(math.radians(self.a - contactAngle)) * math.sin(math.radians(contactAngle + 90))
            other.vx = (other.v * math.cos(math.radians(other.a - contactAngle)) * (other.mass - self.mass) + 2 * self.mass * originalVx * math.cos(math.radians(self.a - contactAngle))) / (self.mass + other.mass) * math.cos(math.radians(contactAngle)) + other.v * math.sin(math.radians(other.a - contactAngle)) * math.cos(math.radians(contactAngle + 90))
            other.vy = (other.v * math.cos(math.radians(other.a - contactAngle)) * (other.mass - self.mass) + 2 * self.mass * originalVy * math.cos(math.radians(self.a - contactAngle))) / (self.mass + other.mass) * math.sin(math.radians(contactAngle)) + other.v * math.sin(math.radians(other.a - contactAngle)) * math.sin(math.radians(contactAngle + 90))
            self.a = math.degrees(math.atan2(self.vy, self.vx))
            other.a = math.degrees(math.atan2(other.vy, other.vx))
            self.v = (self.vx ** 2 + self.vy ** 2) ** 0.5
            other.v = (other.vx ** 2 + other.vy ** 2)  ** 0.5

            # If the balls are overlapping, move them apart
            if distance < self.radius + other.radius:
                self.x -= (self.radius + other.radius - distance) * math.cos(math.radians(contactAngle))
                self.y -= (self.radius + other.radius - distance) * math.sin(math.radians(contactAngle))
                other.x += (self.radius + other.radius - distance) * math.cos(math.radians(contactAngle))
                other.y += (self.radius + other.radius - distance) * math.sin(math.radians(contactAngle))
            

pygame.init()
screen = pygame.display.set_mode(screenSize)
pygame.display.set_caption("Ball Collisions")
clock = pygame.time.Clock()

balls = [Ball((screenSize[0] - ballSize * 2) * i / horizontalAmount + ballSize, (screenSize[1] - ballSize * 2) * j / verticalAmount + ballSize, 0, 0, ballSize) for i in range(horizontalAmount) for j in range(verticalAmount)]
balls.append(Ball(1160, 560, 750, -135, 20))

while True:
    for event in pygame.event.get():
        if event.type == pygame.QUIT:
            sys.exit()

    screen.fill((0, 0, 0))

    for ball in balls:
        ball.move(1 / fps)
        ball.draw(screen)

    for i in range(len(balls)):
        for j in range(i + 1, len(balls)):
            balls[i].collide(balls[j])

    pygame.display.flip()
    clock.tick(fps)