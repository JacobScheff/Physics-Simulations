import pygame
import sys
import time
import math

screenSize = (1200, 600)

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

    def move(self):
        self.x += self.vx
        self.y += self.vy

        if self.x < self.radius or self.x > screenSize[0] - self.radius:
            self.vx *= -1
        if self.y < self.radius or self.y > screenSize[1] - self.radius:
            self.vy *= -1

        self.a = math.degrees(math.atan2(self.vy, self.vx))

    def checkCollision(self, other):
        if (self.x - other.x) ** 2 + (self.y - other.y) ** 2 <= (self.radius + other.radius) ** 2:
            return True
        else:
            return False
    
    def collide(self, other):
        if self.checkCollision(other):
            originalVx = self.vx
            originalVy = self.vy
            contactAngle = math.degrees(math.atan2(other.y - self.y, other.x - self.x))
            
            self.vx = (self.v * math.cos(math.radians(self.a - contactAngle)) * (self.mass - other.mass) + 2 * other.mass * other.v * math.cos(math.radians(other.a - contactAngle))) / (self.mass + other.mass) * math.cos(math.radians(contactAngle)) + self.v * math.sin(math.radians(self.a - contactAngle)) * math.cos(math.radians(contactAngle + 90))
            self.vy = (self.v * math.cos(math.radians(self.a - contactAngle)) * (self.mass - other.mass) + 2 * other.mass * other.v * math.cos(math.radians(other.a - contactAngle))) / (self.mass + other.mass) * math.sin(math.radians(contactAngle)) + self.v * math.sin(math.radians(self.a - contactAngle)) * math.sin(math.radians(contactAngle + 90))
            other.vx = (other.v * math.cos(math.radians(other.a - contactAngle)) * (other.mass - self.mass) + 2 * self.mass * originalVx * math.cos(math.radians(self.a - contactAngle))) / (self.mass + other.mass) * math.cos(math.radians(contactAngle)) + other.v * math.sin(math.radians(other.a - contactAngle)) * math.cos(math.radians(contactAngle + 90))
            other.vy = (other.v * math.cos(math.radians(other.a - contactAngle)) * (other.mass - self.mass) + 2 * self.mass * originalVx * math.cos(math.radians(self.a - contactAngle))) / (self.mass + other.mass) * math.sin(math.radians(contactAngle)) + other.v * math.sin(math.radians(other.a - contactAngle)) * math.sin(math.radians(contactAngle + 90))

            self.a = math.degrees(math.atan2(self.vy, self.vx))
            other.a = math.degrees(math.atan2(other.vy, other.vx))
            self.v = math.sqrt(self.vx ** 2 + self.vy ** 2)
            other.v = math.sqrt(other.vx ** 2 + other.vy ** 2)

pygame.init()
screen = pygame.display.set_mode(screenSize)
pygame.display.set_caption("Ball Collisions")
clock = pygame.time.Clock()

ballSize = 20
horizontalAmount = 20
verticalAmount = 10
balls = [Ball((screenSize[0] - ballSize * 2) * i / horizontalAmount + ballSize, (screenSize[1] - ballSize * 2) * j / verticalAmount + ballSize, 0, 0, ballSize) for i in range(horizontalAmount) for j in range(verticalAmount)]
balls.append(Ball(1160, 560, 500, 135, 20))

while True:
    for event in pygame.event.get():
        if event.type == pygame.QUIT:
            sys.exit()

    screen.fill((0, 0, 0))

    for ball in balls:
        ball.move()
        ball.draw(screen)

    for i in range(len(balls)):
        for j in range(i + 1, len(balls)):
            balls[i].collide(balls[j])

    pygame.display.flip()
    clock.tick(100)