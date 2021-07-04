from shapely.geometry import Point
from route3_client import Server

disturbance = Point(10.86, 48.38).buffer(0.015)
target_points = [
    Point(10.903, 48.348263),
    Point( 10.86698055267334, 48.465011068126444),
]

server = Server()
resp = server.analyze_disturbance(disturbance, 5000.0, target_points)
print(resp.population_within_disturbance)
