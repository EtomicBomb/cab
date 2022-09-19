# Courses with interesting profiles

* NEUR 2110: lost prerequisite restrictions fall 2021 randomly, then regained them
* VISA 1800C: prereq, cls, prg, lvl restrictions
* POLS 1822G: has semester level (cls), program (prg), and grad/undergrad restrictions (lvl)
* UNIV 1702: has prerequisite (prereq) and semester level (cls) restrictions
* VISA 1800P: has prerequisite (prereq), semester level (cls), and program (prg) restrictions
* VISA 1510: circular dependencies
* VISA 1110: two sections, different professors? not sure why i included
* FIX ITALIAN, FIX APMA 1160, FIX FREN 0400, fix HISP 0200 HISP 0300 not recognize cab
* latn 200 0100 and 110 have two any's
* HIST 0930B: Students must register under the CRN of an appropriate course
* BIOL XLIST: watch out for xlist courses
* HISP 0750G: first year seminar, no mentioned level requirement.
* BIOL 0940A: enrollment mentioned in two places
* LACA 0510R: does not have its synonym course code in the registration notes.
* LATN 0200, 0100, 110 have two any's

# Notes

* FIX ITALIAN, FIX APMA 1160, FIX FREN 0400, fix HISP 0200 HISP 0300 not recognize cab
* Examples of informal prerequisites include auditions, demonstrated experience in the field, some programming experience, a specific level of knowledge in a foreign language, etc.

https://www.brown.edu/academics/math/course-descriptions
* https://selfservice.brown.edu/ss/bwckctlg.p_disp_dyn_ctlg
* going all the way back to 2013 https://selfservice.brown.edu/ss/bwckctlg.p_display_courses
* HIST courses numbers: 0 - undergrad only, 1 - accessible to all students, 2 -> grad only
* `.stat == "X"` means the exact same thing as `.section == ""`
* if the title contains a course code, it refers to its cannonical name

# Interesting jq queries

## Courses that lost prerequisites:

```
jq -s 'map(select(.section | startswith("S"))) | group_by(.code) | .[] | reduce .[] as $c ({p:false,w:false}; {code:$c.code, p:(.p or ($c.registration_restrictions | contains("prereq"))), w:(.w or (.p and ($c.registration_restrictions | contains("prereq") | not)))}) | select(.w)' all.json
jq 'select((.section == "") and (.code | contains("XLIST") | not)  and (.description | (startswith("Interested")) | not))' all.json
```

# Information available

* In the box
    * Subject
    * Number
    * Course offering semester (spring, summer, fall, winter)
* Hover
    * Title (hover?)
    * Description (hover?) 
    * How many people took it?
    * Are there unmentioned prerequisites?
    * Graduate / Undergraduate level restrictions
    * Semester Level Restrictions
        * Collapse semester level 01-02 restrictions into 'freshman' restrictions, etc?
    * Average, max hours
    * Program: sophomore seminar, writ, race, power and privilage?
* Outside Box
    * Prerequisite graph

# Kinds of prerequisites

* Course 
* Exam score
* Named group (linear algebra)
* Conjunction of prerequisites
* Disjunction of prerequisites

# Purpose

This application helps students quickly find information about Brown courses they might be interested in.
Brown courses that might interest them. 

# Failure model

TODO

# Processing

## Stage 1

Input: undocumented cab.brown.edu API
Output: cab.jsonl file

Pulls all available course data.

## Stage 2

Input: cab.jsonl
Input: correction files
Output: courses.jsonl

Processes course data into a form that's convenient for the server. Produces a JSON lines file,
with each record containing the course code and the available course information. Course information
is in its final processed form. The prerequisite string is minimized, 

The most recent present qualification is chosen to be the cannonical one for the course. 

```json
{
    "code": "BIOL 0320A",
    "title": "The Origin of Life On Earth",
    "description": "Learn about how life arose on this planet with the help of aliens",
    "prerequisites": {
        "all": [
            {"course": "MATH 0100"},
            {"course": "MATH 0200"},
            {
                "any": [
                    {"course": "ENGN 0032"},                
                    {"exam": "Engineering Placement", "score": 720},
                ]
            }
        ]
    },
    "semester": {"start": 3, "end": 14}, // nullable
    "restricted": true,
    "aliases": [],
    "offerings": [ // most recent first?
        {
            "date": "Spring 2022",
            "section": 1,
            "instructors": ["Bill Michaelson"],
            "enrollment": 40,
            "demographics": { // nullable
                "freshman": 2,
                "sophomores": 4, 
                "juniors": 18,
                "seniors": 32,
                "others": 0,
            },
            "review": { // nullable, issue: review is associated with a date, not with date, section
                "average": 8,
                "max": 13,
                "course": 4.0,
                "professor": 4.5,
            },
        },
    ],
}
```


### Minimizing the prerequisite graph

Removes unnecessary edges in the prerequisite graph by minimizing a logic expression encoding 
the prerequisites of every Brown course.

courses.all(|(code, prereqs)| code ⇒ prereqs)

Find an equivalent expression in conjuctive normal form. Replace all (¬c ∨ p) with (c ⇒ p).
Each of these terms represent an arrow in the prerequisite graph.

#### Rules:

before | after | name
---|---|---
a->(B B C) | a->(B C) | idempotency
a->(a B) |  | negation
a->(b) b->(C) a->(C D) | a->(b) b->(C) | transitivity
a->(B) c->(a B D) | a->(B) c->(B D) | absorption 
a->(B) a->(B C) | a->(B) | simplify

## Stage 3: Visualization

Visualization should probably be a svg because this can be both converted to a pdf and interacted with
on the web. 

We know how many people have taken a given course, so common prerequisites can be highlighted?

We can group common disjunctions of prerequisites into named groups that can be clicked and expanded?

Users could select courses to add individually, by concentration, or by subject. Their prerequisites
and relationships could be shown. Courses that were not searched for could be in much smaller boxes
than the relevant ones.

Prerequisite courses could be duplicated (there's no reason for all successor courses to link back to a 
single instance of the course in a box).

A course could be centered in the visualization - all of its children and parents are shown
as coming out of one box. Non-centered courses in the same visualization could be subject to 
duplication, being shrinked, etc.



