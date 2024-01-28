import pygame
import sys
import time

screenSize = (600, 300)

class Ball:
    def __init__(self, x, y, vx, vy, radius):
        self.x = x
        self.y = y
        self.vx = vx
        self.vy = vy
        self.radius = radius

    def draw(self, screen):
        pygame.draw.circle(screen, (255, 255, 255), (self.x, self.y), self.radius)

    def move(self):
        self.x += self.vx
        self.y += self.vy

        if self.x < self.radius or self.x > screenSize[0] - self.radius:
            self.vx *= -1
        if self.y < self.radius or self.y > screenSize[1] - self.radius:
            self.vy *= -1

    def checkCollision(self, other):
        if (self.x - other.x) ** 2 + (self.y - other.y) ** 2 <= (self.radius + other.radius) ** 2:
            return True
        else:
            return False
    
    def collide(self, other):
        if self.checkCollision(other):
            self.vx *= -1
            self.vy *= -1
            other.vx *= -1
            other.vy *= -1

pygame.init()
screen = pygame.display.set_mode(screenSize)
pygame.display.set_caption("Bouncing Ball")
clock = pygame.time.Clock()

balls = [Ball(100, 100, 1, 1, 10), Ball(200, 200, 2, 2, 20), Ball(300, 200, 3, 3, 30), Ball(500, 200, 4, 4, 40)]


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