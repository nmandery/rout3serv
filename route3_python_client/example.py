from shapely.geometry import Point
from route3_client import Server

disturbance = Point(8.9, 49.43).buffer(0.015)
target_points = [
    Point(8.92, 49.412),
    Point(8.88, 49.2),
]

server = Server()
resp = server.analyze_disturbance(disturbance, 5000.0, target_points)
print(resp.population_within_disturbance)
