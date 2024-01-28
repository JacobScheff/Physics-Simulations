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
        self.v = (self.vx ** 2 + self.vy ** 2) ** 0.5
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

    def checkCollision(self, other):
        if (self.x - other.x) ** 2 + (self.y - other.y) ** 2 <= (self.radius + other.radius) ** 2:
            return True
        else:
            return False
    
    def collide(self, other):
        if self.checkCollision(other):
            originalVx = self.vx
            originalVy = self.vy
            self.vx = (self.mass - other.mass) / (self.mass + other.mass) * self.vx + 2 * other.mass / (self.mass + other.mass) * other.vx
            self.vy = (self.mass - other.mass) / (self.mass + other.mass) * self.vy + 2 * other.mass / (self.mass + other.mass) * other.vy
            other.vx = 2 * self.mass / (self.mass + other.mass) * originalVx + (other.mass - self.mass) / (self.mass + other.mass) * other.vx
            other.vy = 2 * self.mass / (self.mass + other.mass) * originalVy + (other.mass - self.mass) / (self.mass + other.mass) * other.vy
            self.v = (self.vx ** 2 + self.vy ** 2) ** 0.5
            other.v = (other.vx ** 2 + other.vy ** 2) ** 0.5


pygame.init()
screen = pygame.display.set_mode(screenSize)
pygame.display.set_caption("Ball Collisions")
clock = pygame.time.Clock()

balls = [Ball(200, 200, 0, 0, 20), Ball(300, 190, 2, 0, 20)]


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