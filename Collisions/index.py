import pygame
import sys
import time
import math
import random

screenSize = (1200, 600)
fps = 25
horizontalCells = 24 # 48
verticalCells = 12 # 24
gravity = 0 # 200
balls = []
ballIndexKey = [[-1, -1]for i in range(horizontalCells * verticalCells)]

# Ball parameters
ballSize = 10
horizontalAmount = 48
verticalAmount = 24
def getRandomVelocity():
    return Vector(0, 0)
    # return Vector(random.randint(-100, 100), random.randint(-100, 100))

class Vector:
    def __init__(self, x, y):
        self.x = x
        self.y = y

    def crossProduct(self, other):
        return self.x * other.y - self.y * other.x
    
    def dotProduct(self, other):
        return self.x * other.x + self.y * other.y
    
    def add(self, other):
        newVector = Vector(self.x + other.x, self.y + other.y)
        return newVector

    def subtract(self, other):
        newVector = Vector(self.x - other.x, self.y - other.y)
        return newVector

    def multiply(self, scalar):
        newVector = Vector(self.x * scalar, self.y * scalar)
        return newVector

    def divide(self, scalar):
        newVector = Vector(self.x / scalar, self.y / scalar)
        return newVector

    def getMagnitude(self):
        return (self.x ** 2 + self.y ** 2) ** 0.5
    
    def getAngle(self):
        return math.degrees(math.atan2(self.y, self.x))
    
    def normalize(self):
        return self.divide(self.getMagnitude())
    
    def setMagnitude(self, magnitude):
        self.normalize().multiply(magnitude)
        return self

class Ball:
    def __init__(self, x, y, vector, radius):
        self.x = x
        self.y = y
        self.vector = vector
        self.radius = radius
        # Mass is the area of the ball
        self.mass = self.radius ** 2 * 3.14

    def draw(self, screen):
        pygame.draw.circle(screen, (255, 255, 255), (self.x, self.y), self.radius)

    def move(self, dt):
        # Apply gravity
        self.vector.y += gravity * dt

        # Move the ball
        self.x += self.vector.x * dt
        self.y += self.vector.y * dt

        # Apply border collision
        if self.x < self.radius:
            self.vector.x = abs(self.vector.x)
            self.x = self.radius
        elif self.x > screenSize[0] - self.radius:
            self.vector.x = -abs(self.vector.x)
            self.x = screenSize[0] - self.radius
        if self.y < self.radius:
            self.vector.y = abs(self.vector.y)
            self.y = self.radius
        elif self.y > screenSize[1] - self.radius:
            self.vector.y = -abs(self.vector.y)
            self.y = screenSize[1] - self.radius
    
    def collide(self, other):
        distance = ((self.x - other.x) ** 2 + (self.y - other.y) ** 2) ** 0.5
        if distance == 0:
            return False
        if distance <= self.radius + other.radius:            
            originalVectorSelf = Vector(self.vector.x, self.vector.y)
            originalVectorOther = Vector(other.vector.x, other.vector.y)
            selfPosition = Vector(self.x, self.y)
            otherPosition = Vector(other.x, other.y)
            totalMass = self.mass + other.mass

            self.vector = originalVectorSelf.subtract(selfPosition.subtract(otherPosition).normalize().multiply(2 * other.mass / totalMass).multiply(originalVectorSelf.subtract(originalVectorOther).dotProduct(selfPosition.subtract(otherPosition))).divide(distance ** 2))
            other.vector = originalVectorOther.subtract(otherPosition.subtract(selfPosition).normalize().multiply(2 * self.mass / totalMass).multiply(originalVectorOther.subtract(originalVectorSelf).dotProduct(otherPosition.subtract(selfPosition))).divide(distance ** 2))

            # If the balls are overlapping, move them apart
            if distance < self.radius + other.radius:
                contactAngle = math.degrees(math.atan2(self.y - other.y, self.x - other.x))
                distanceToMove = (self.radius + other.radius - distance)
                # bigger mass == less movement
                self.x += distanceToMove * math.cos(math.radians(contactAngle)) * other.mass / totalMass
                self.y += distanceToMove * math.sin(math.radians(contactAngle)) * other.mass / totalMass
                other.x -= distanceToMove * math.cos(math.radians(contactAngle)) * self.mass / totalMass
                other.y -= distanceToMove * math.sin(math.radians(contactAngle)) * self.mass / totalMass
            # print("v1:\t" + str(self.vector.getMagnitude()) + "\tv2:\t" + str(other.vector.getMagnitude()) + "\tangle:\t" + str(math.degrees(math.atan2(self.y - other.y, self.x - other.x))))
            
            return True
        return False
    
    def getCell(self):
        x = int(min(max(self.x // (screenSize[0] / horizontalCells), 0), horizontalCells - 1))
        y = int(min(max(self.y // (screenSize[1] / verticalCells), 0), verticalCells - 1))
        return (x, y)
    
    def getCellId(self):
        return self.getCell()[0] + self.getCell()[1] * horizontalCells
    
# Get dot product of two vectors
def dotProduct(v1, a1, v2, a2):
    return v1 * v2 * math.cos(math.radians(a1 - a2))
    

for i in range(horizontalAmount):
    for j in range(verticalAmount):
        ballPos = (screenSize[0] - ballSize * 2) * i / horizontalAmount + ballSize, (screenSize[1] - ballSize * 2) * j / verticalAmount + ballSize
        randomVector = getRandomVelocity()
        balls.append(Ball(ballPos[0], ballPos[1], randomVector, ballSize))

# balls.append(Ball(200, 200, Vector(0, 0), 20))
# balls.append(Ball(300, 300, Vector(-150, -150), 20))
balls.append(Ball(1120, 500, Vector(-1000, -400), 40))

pygame.init()
screen = pygame.display.set_mode(screenSize)
pygame.display.set_caption("Ball Collisions")
clock = pygame.time.Clock()

# Use a binary search to get the first index of the ball with the target cell id
def binarySearchBallIndexFirst(arr, targetCellId, start=0, end=len(balls) - 1):
    loops = 0
    while start <= end:
        loops += 1
        mid = (start + end) // 2
        if arr[mid].getCellId() == targetCellId:
            # Get the first index with the same cell id
            for i in range(mid, -1, -1):
                if arr[i].getCellId() != targetCellId:
                    return i + 1
            return 0
        elif arr[mid].getCellId() < targetCellId:
            start = mid + 1
        else:
            end = mid - 1
    return -1

# Use a binary search to get the last index of the ball with the target cell id
def binarySearchBallIndexLast(arr, targetCellId, start=0, end=len(balls) - 1):
    loops = 0
    while start <= end:
        loops += 1
        mid = (start + end) // 2
        if arr[mid].getCellId() == targetCellId:
            # Get the last index with the same cell id
            for i in range(mid, len(arr)):
                if arr[i].getCellId() != targetCellId:
                    return i - 1
            return len(arr) - 1
        elif arr[mid].getCellId() < targetCellId:
            start = mid + 1
        else:
            end = mid - 1
    return -1

# Insertion sort the balls (possibly can use binary sort to make this even faster)
def sortBalls():
    for i in range(1, len(balls)):
        key = balls[i]
        keyId = key.getCellId()
        j = i - 1
        while j >= 0 and balls[j].getCellId() > keyId:
            balls[j + 1] = balls[j]
            j -= 1
        balls[j + 1] = key
    # Update the ball index key
    startIndex = 0
    for i in range(horizontalCells * verticalCells):
        foundIndexStart = binarySearchBallIndexFirst(balls, i, start=startIndex)
        foundIndexEnd = binarySearchBallIndexLast(balls, i, start=startIndex)
        if foundIndexStart!= -1:
            startIndex = foundIndexStart
        ballIndexKey[i] = [foundIndexStart, foundIndexEnd]

fpsTimer = time.time()
while True:
    for event in pygame.event.get():
        if event.type == pygame.QUIT:
            sys.exit()

    screen.fill((0, 0, 0))

    # Sort the balls by cell id
    sortBalls()

    # Move and draw the balls
    for ball in balls:
        ball.draw(screen)
        ball.move(1 / fps)

    # Check for collisions in the current cell and the adjacent cells
    # ballsAlreadyChecked = []
    for cellX in range(horizontalCells):
        for cellY in range(verticalCells):
            cellId = cellX + cellY * horizontalCells
            for ballIndex in range(ballIndexKey[cellId][0], ballIndexKey[cellId][1] + 1):
                for j in range(-1, 2):
                    for k in range(-1, 2):
                        if cellX + j >= 0 and cellX + j < horizontalCells and cellY + k >= 0 and cellY + k < verticalCells:
                            newCellId = cellX + j + (cellY + k) * horizontalCells
                            for otherBallIndex in range(ballIndexKey[newCellId][0], ballIndexKey[newCellId][1] + 1):
                                # There are no balls in this cell
                                if(ballIndex == -1 or otherBallIndex == -1):
                                    continue
                                if ballIndex != otherBallIndex:
                                    # Check if the balls have already been checked
                                    # if (ballIndex, otherBallIndex) in ballsAlreadyChecked or (otherBallIndex, ballIndex) in ballsAlreadyChecked:
                                    #     continue
                                    # ballsAlreadyChecked.append((ballIndex, otherBallIndex))
                                    balls[ballIndex].collide(balls[otherBallIndex])

    pygame.display.flip()
    clock.tick(fps)
    if time.time() - fpsTimer >= 1:
        totalKineticEnergy = 0
        for ball in balls:
            totalKineticEnergy += ball.mass * ball.vector.getMagnitude() ** 2
        print(str(clock.get_fps()) + "\t" + str(totalKineticEnergy))
        fpsTimer = time.time()