const fs = require("fs");

let LUTS = [];
for (let i=0;i<100;i++) {
    LUTS.push([]);
}

let PACKED_LUT = new BigInt64Array(1000*76);

function solve(value,n) {
    if (LUTS[n][value] != null) {
        return LUTS[n][value];
    }

    if (n==0) {
        return 1;
    }
    if (value == 0) {
        return solve(1,n-1);
    } else {
        let str = value.toString();
        if (str.length%2 == 0) {
            let a = str.substring(0,str.length/2);
            let b = str.substring(str.length/2);
            return solve(+a,n-1) + solve(+b,n-1);
        } else {
            return solve(value * 2024,n-1);
        }
    }
}

const LUT_SIZE = 1000;

let next_lut_index = 0;
for (let i=0;i<=75;i++) {
    for (let j=0;j<LUT_SIZE;j++) {
        LUTS[i][j] = solve(j,i);
        PACKED_LUT[next_lut_index] = BigInt(LUTS[i][j]);
        next_lut_index++;
    }
    console.log("finished level",i);
}

fs.writeFileSync("src/day11_lut.bin",PACKED_LUT);
