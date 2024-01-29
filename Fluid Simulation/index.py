import pygame
import sys
import time
import math

screenSize = (1200, 600)
ballRadius = 10
horizontalAmount = 1
verticalAmount = 20
fps = 240

class Ball:
    def __init__(self, x, y, vx, vy):
        self.x = x
        self.y = y
        self.vx = vx
        self.vy = vy
        self.radius = ballRadius

    def draw(self, screen):
        pygame.draw.circle(screen, (255, 255, 255), (self.x, self.y), ballRadius)

    def move(self, dt):
        # If the ball is going to hit the wall, reverse the velocity
        if self.x < self.radius or self.x > screenSize[0] - self.radius:
            self.vx *= -1
        if self.y < self.radius or self.y > screenSize[1] - self.radius:
            self.vy *= -1

        # Move the ball
        self.x += self.vx * dt
        self.y += self.vy * dt
            
pygame.init()
screen = pygame.display.set_mode(screenSize)
pygame.display.set_caption("Fluid Simulation")
clock = pygame.time.Clock()

# balls = [Ball((screenSize[0] - ballRadius * 2) * i / (horizontalAmount - 1) + ballRadius, (screenSize[1] - ballRadius * 2) * j / (verticalAmount - 1) + ballRadius, 0, 0) for i in range(horizontalAmount) for j in range(verticalAmount)]
balls = []
for i in range(horizontalAmount):
    for j in range(verticalAmount):
        padding = 100
        y = (screenSize[1] - padding - ballRadius * 2) * j / (verticalAmount - 1) + ballRadius - (padding / 2)
        balls.append(Ball(20, y, 0, 0))

while True:
    for event in pygame.event.get():
        if event.type == pygame.QUIT:
            sys.exit()

    screen.fill((0, 0, 0))

    for ball in balls:
        ball.move(1 / fps)
        ball.draw(screen)

    pygame.display.flip()
    clock.tick(fps)