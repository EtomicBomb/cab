A: 
B: A
C: any(A B) = any(A all(A B)) = A

----------------------------------------------------

A: 
B: A
C: all(A B) = all(A all(B A)) = all(A B A) = all(A B) = B

----------------------------------------------------

irreducible
A: 
B: 
C: any(A B)
D: 
E: any(D A C) 

----------------------------------------------------

A: 
B: 
C: all(A B)
D: 
E: all(D A C) = all(D A all(C all(A B))) = all(D A C A B) = all(D C A B) = all(D all(C all(A B))) = all(D C)

----------------------------------------------------

A: 
B: 
C: 
D: 
E: all(A B any(C D))
F: 
G: all(A B E F) = all(A B all(E all(A B any(C D))) F) = all(A B E A B F any(C D)) = all(F E A B any(C D)) = all(F all(E all(A B any(C D)))) = all(F E)

----------------------------------------------------

A:
B: A
C: all(A B) = B
D: all(A B C) = all(C all(A B)) = all(C B) = C

----------------------------------------------------

A:
B:
C: any(A B)
D: any(A B C) = any(A B all(C any(A B))) = any(A B any(and(C A) and(C B))) = any(A B and(C A) and(C B)) = any(any(A and(C A)) any(B all(C B))) = any(A B)
