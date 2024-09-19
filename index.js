const fs = require('fs');

let rawData = fs.readFileSync('abc.txt', 'utf8');
rawData = rawData.split('\n');

let data = "";

// Morning
// for (let i = 0; i < rawData.length; i++) {
//     let morningSplit = rawData[i].split('morning');
//     for(let j = 0; j < morningSplit.length; j++) {
//         if (morningSplit[j].split("/").length != 2) {
//             let date = rawData[i].split(" ")[0];
//             let right = morningSplit[j].split(" ").filter((item) => item != '')[0];
//             if (right.split("/")[0] > 12){
//                 let left = morningSplit[j].split(" ").filter((item) => item != '')[2];
//                 if(left != "noon" && left != "night"){
//                     data += `${date}, ${left.split("/")[0]}, ${left.split("/")[1]}\n`;
//                 }
//                 // else {
//                     // data += `${date}, ${right.split("/")[0]}, ${right.split("/")[1]}\n`;
//                 // }
//             }
//         }
//     }
// }

// Night
for (let i = 0; i < rawData.length; i++) {
    let morningSplit = rawData[i].split('night');
    for(let j = 0; j < morningSplit.length; j++) {
        if (morningSplit[j].split("/").length != 2) {
            let date = rawData[i].split(" ")[0];
            let right = morningSplit[j].split(" ").filter((item) => item != '')[0];
            if (right.split("/")[0] > 12){
                let left = morningSplit[j].split(" ").filter((item) => item != '')[2];
                if(left != "noon" && left != "morning"){
                    data += `${date}, ${left.split("/")[0]}, ${left.split("/")[1]}\n`;
                }
                // else {
                    // data += `${date}, ${right.split("/")[0]}, ${right.split("/")[1]}\n`;
                // }
            }
        }
    }
}

console.log(data);